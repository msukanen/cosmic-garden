//! Roomies.

use std::{collections::HashMap, fmt::Display, fs as sync_fs, sync::{Arc, Weak}};

use cosmic_garden_pm::{DescribableMut, IdentityMut};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, fs as async_fs};

use crate::{error::CgError, identity::IdentityQuery, io::room_fp, item::container::variants::{ContainerVariant, ContainerVariantType}, mob::core::Entity, player::Player, string::Slugger, traits::Tickable, util::direction::Direction, world::World};

#[derive(Debug, Clone)]
pub enum RoomError {
    NoSuchRoom(String),
}

impl Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchRoom(id) => write!(f, "No such room (yet) in existence as {}", id)
        }
    }
}

impl std::error::Error for RoomError {}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, DescribableMut)]
pub struct Room {
    id: String,
    title: String,
    #[serde(default = "empty_room_desc")]
    pub desc: String,
    #[serde(default, skip)]
    pub who: HashMap<String, Weak<RwLock<Player>>>,

    #[serde(default, skip)]
    pub exits: HashMap<Direction, Weak<RwLock<Room>>>,
    
    #[serde(default)]
    pub raw_exits: HashMap<Direction, String>,

    #[serde(default = "room_inventory")]
    pub contents: ContainerVariant,

    /// NPC [entities][Entity] in the [Room].
    // [Room] is the sole owner of an [Entity].
    #[serde(default, with = "arc_n_t_transform")]
    pub entities: HashMap<String, Arc<RwLock<Entity>>>,
}

fn empty_room_desc() -> String { "A room.".into() }
fn room_inventory() -> ContainerVariant { ContainerVariant::raw(ContainerVariantType::Room) }

mod arc_n_t_transform {
    use std::{collections::HashMap, sync::Arc};

    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeMap};
    use tokio::sync::RwLock;

    use crate::mob::core::Entity;

    pub fn serialize<S>(what: &HashMap<String, Arc<RwLock<Entity>>>, s:S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut map = s.serialize_map(Some(what.len()))?;
        for (id, arc) in what {
            // try_read - skip if contested atm.
            if let Ok(guard) = arc.try_read() {
                map.serialize_entry(id, &*guard)?;
            } else {
                // skip for now, Janitor'll get this done sooner or later…
                log::debug!("Skipping {id} in save: lock busy.")
            }
        }
        map.end()
    }

    pub fn deserialize<'de, D>(d: D) -> Result< HashMap<String, Arc<RwLock<Entity>>>, D::Error>
    where D: Deserializer<'de>
    {
        let raw: HashMap<String, Entity> = HashMap::deserialize(d)?;
        let mut arced = HashMap::with_capacity(raw.len());
        for (id, ent) in raw {
            arced.insert(id, Arc::new(RwLock::new(ent)));
        }

        Ok(arced)
    }
}

impl Room {
    pub fn load_sync(id: &str) -> Result<Self, CgError> {
        let room: Room = serde_json::from_str(
            &sync_fs::read_to_string(room_fp(id))?
        )?;
        Ok(room)
    }

    pub async fn new(id: &str, title: &str) -> Result<Arc<RwLock<Self>>, CgError> {
        // check if there is pre-existing file...
        let room = Room::load_sync(id).unwrap_or({
            log::info!("No archælogy possible, thus creating new room '{}'", id);
            Self {
                id: id.as_id()?,
                title: title.into(),
                desc: empty_room_desc(),
                who: HashMap::new(),
                exits: HashMap::new(),
                raw_exits: HashMap::new(),
                contents: room_inventory(),
                entities: HashMap::new(),
            }
        });

        Ok(Arc::new(RwLock::new(room)))
    }

    pub async fn save(&self) -> Result<(), CgError> {
        let path = room_fp(&self.id);
        log::debug!("Saving '{}'…", path.display());
        async_fs::write(path, serde_json::to_string_pretty(self)?).await?;
        Ok(())
    }

    pub async fn link_exit(&mut self, world: Arc<RwLock<World>>, dir: Direction, target_id: &str) -> Result<(), CgError> {
        log::debug!("Linking '{}'({dir}) to '{target_id}'…", self.id());
        if let Some(_) = self.raw_exits.insert(dir.clone(), target_id.into()) {
            log::warn!("Overriding already existing '{dir}'.");
        }
        // Find the target room, hopefully.
        let w = world.read().await;
        let my_lock = if let Some(my_arc) = w.rooms.get(self.id()) {
            Arc::downgrade(my_arc)
        } else {
            log::error!("Wait what? Where did '{}' lock go?!", self.id());
            return Err(CgError::from(RoomError::NoSuchRoom(self.id().to_string())))
        };
        if let Some(target_arc) = w.rooms.get(target_id) {
            self.exits.insert(dir.clone(), Arc::downgrade(target_arc));
            log::debug!("Real link established between '{}' and '{}'", self.id(), target_id);
            log::debug!("Attempting reverse…");
            if let Ok(opp_dir) = dir.opposite() {
                let mut tgt_lock = target_arc.write().await;
                tgt_lock.exits.insert(opp_dir.clone(), my_lock);
                tgt_lock.raw_exits.insert(opp_dir.clone(), self.id().into());
                log::debug!("Symmetry check OK {dir} ↔ {opp_dir}");
            } else {
                log::warn!("One-way grit: Direction {dir:?} is non-reversible. Good luck, traveller!");
            }
            return Ok(())
        }

        // Exits are allowed to point to non-existing ways… Mirage entrances, etc.
        self.exits.insert(dir.clone(), Weak::new());
        Err(CgError::from(RoomError::NoSuchRoom(target_id.into())))
    }

    pub fn shallow_clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            title: self.title.clone(),
            desc: self.desc.clone(),
            exits: self.exits.clone(),
            raw_exits: self.raw_exits.clone(),
            // we skip everything else:
            who: HashMap::new(),
            contents: room_inventory(),
            entities: HashMap::new(),
        }
    }

    pub fn copyback(&mut self, source: Self) {
        self.id = source.id;
        self.title = source.title;
        self.desc = source.desc;
        self.exits = source.exits;
        self.raw_exits = source.raw_exits;
    }
}

#[cfg(test)]
mod room_tests {
    use crate::{util::direction::Direction, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn room_linking() {
        let (w_arc,c,p,_) = get_operational_mock_world().await;
        w_arc.write().await.link_rooms().await;
        let rooms = w_arc.read().await.rooms.clone();
        if let Some(r_arc) = rooms.get("room-1") {
            let mut r = r_arc.write().await;
            if let Err(e) = r.link_exit(w_arc.clone(), Direction::North, "room-2_".into()).await {
                panic!("Bummer… {e:?}");
            }
        }
        for r in &rooms {
            log::debug!("Room {r:?}")
        }
        // this should fail symmetry check:
        if let Some(r_arc) = rooms.get("room-1") {
            let mut r = r_arc.write().await;
            if let Err(e) = r.link_exit(w_arc.clone(), Direction::Custom("trampoline".into()), "room-2_".into()).await {
                panic!("Bummer… {e:?}");
            }
        }
        for r in &rooms {
            log::debug!("Room {r:?}")
        }
        // this should create a "mirage" and override an old entry:
        if let Some(r_arc) = rooms.get("room-1") {
            let mut r = r_arc.write().await;
            if let Err(e) = r.link_exit(w_arc.clone(), Direction::Custom("trampoline".into()), "room-3".into()).await {
                log::error!("Bummer… {e:?}");
            }
        }
        for r in &rooms {
            log::debug!("Room {r:?}")
        }
    }
}

impl Room {
    pub async fn tick(&mut self) {
        for p_weak in self.who.values() {
            if let Some(p_arc) = p_weak.upgrade() {
                let mut p = p_arc.write().await;
                // no reaction yet to "positive" tick(s)
                let _= p.tick().await;
            }
        }

        for e in self.entities.values() {
            let mut lock = e.write().await;
            // no reaction yet to "positive" tick(s)
            let _ = lock.tick().await;
        }

        // no reaction yet to "positive" tick(s)
        let _ = self.contents.tick().await;
        #[cfg(all(debug_assertions,feature = "stresstest"))]{
            log::debug!("Room '{}' ticked.", self.id);
        }
    }
}
