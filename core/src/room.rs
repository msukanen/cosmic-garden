//! Roomies.

use std::{collections::{HashMap, HashSet, VecDeque}, fmt::Display, fs as sync_fs, sync::{Arc, Weak}};

use cosmic_garden_pm::{DescribableMut, IdentityMut};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, fs as async_fs};

use crate::{error::CgError, identity::{IdentityQuery, MachineIdentity, uniq::UuidValidator}, io::room_fp, item::{Item, container::{storage::{Storage, StorageError, StorageMut, StorageQueryError, StorageSpace}, variants::{ContainerVariant, ContainerVariantType}}}, mob::core::Entity, player::Player, room::locking::{Exit, ExitState}, traits::Tickable, util::direction::Direction, world::World};

pub mod locking;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum MemoryFog {
    Jail,
    Mystic,
}

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

macro_rules! option_room_id_from_weak {
    ($r:expr) => {
        if let Some(arc) = $r.upgrade() {
            arc.read().await.id().to_string().into()
        } else { None }
    };
}

impl std::error::Error for RoomError {}

/// Payload for [SystemSignal].
pub enum RoomPayload {
    Id(String),
    Arc(Arc<RwLock<Room>>)
}

impl From<&str> for RoomPayload {
    fn from(value: &str) -> Self {
        Self::Id(value.into())
    }
}

impl From<String> for RoomPayload {
    fn from(value: String) -> Self {
        Self::Id(value)
    }
}

impl From<Arc<RwLock<Room>>> for RoomPayload {
    fn from(value: Arc<RwLock<Room>>) -> Self {
        Self::Arc(value)
    }
}

impl RoomPayload {
    pub async fn id(&self) -> String {
        match self {
            Self::Id(s) => s.to_string(),
            Self::Arc(arc) => arc.read().await.id().into()
        }
    }
}

fn empty_room_desc() -> String { "A room.".into() }
fn room_inventory() -> ContainerVariant { ContainerVariant::raw(ContainerVariantType::Room) }

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, DescribableMut)]
pub struct Room {
    id: String,
    title: String,
    #[serde(default = "empty_room_desc")]
    pub desc: String,
    #[serde(default, skip)]
    pub who: HashMap<String, Weak<RwLock<Player>>>,

    #[serde(default, skip)]
    pub exits: HashMap<Direction, Exit>,
    
    #[serde(default)]
    raw_exits: HashMap<Direction, ExitLike>,

    #[serde(default = "room_inventory")]
    contents: ContainerVariant,

    /// NPC [entities][Entity] in the [Room].
    // [Room] is the sole owner of an [Entity].
    #[serde(default, with = "arc_n_t_transform")]
    pub entities: HashMap<usize, Arc<RwLock<Entity>>>,

    #[serde(default)]
    pub memory_fog: Option<MemoryFog>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExitLike {
    #[serde(default)]
    pub room_id: Option<String>,
    #[serde(default)]
    pub key_bp: Option<String>,
    pub state: ExitState,
}

impl ExitLike {
    pub async fn from(ex: &Exit) -> Self {
        match ex {
            Exit::Closed { key_bp, room } =>
                Self {
                    room_id: option_room_id_from_weak!(room),
                    key_bp: key_bp.clone(),
                    state: ExitState::Closed
                },
            Exit::Free { room } =>
                Self { room_id: option_room_id_from_weak!(room),
                    key_bp: None,
                    state: ExitState::Free
                },
            Exit::Locked { key_bp, room } =>
                Self { room_id: option_room_id_from_weak!(room),
                    key_bp: key_bp.clone().into(),
                    state: ExitState::Locked
                },
            Exit::LockedAL { key_bp, room } =>
                Self { room_id: option_room_id_from_weak!(room),
                    key_bp: key_bp.clone().into(),
                    state: ExitState::LockedAL
                },
            Exit::Open { key_bp, room } =>
                Self { room_id: option_room_id_from_weak!(room),
                    key_bp: key_bp.clone(),
                    state: ExitState::Open
                },
            Exit::OpenAL { key_bp, room } =>
                Self { room_id: option_room_id_from_weak!(room),
                    key_bp: key_bp.clone().into(),
                    state: ExitState::OpenAL
                },
        }
    }
}

mod arc_n_t_transform {
    use std::{collections::HashMap, sync::Arc};

    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeMap};
    use tokio::sync::RwLock;

    use crate::{identity::MachineId, mob::core::Entity};

    pub fn serialize<S>(what: &HashMap<MachineId, Arc<RwLock<Entity>>>, s:S) -> Result<S::Ok, S::Error>
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

    pub fn deserialize<'de, D>(d: D) -> Result< HashMap<MachineId, Arc<RwLock<Entity>>>, D::Error>
    where D: Deserializer<'de>
    {
        let raw: HashMap<MachineId, Entity> = HashMap::deserialize(d)?;
        let mut arced = HashMap::with_capacity(raw.len());
        for (id, ent) in raw {
            arced.insert(id, Arc::new(RwLock::new(ent)));
        }

        Ok(arced)
    }
}

impl Room {
    /// Attempt to load a room.
    /// 
    /// # Args
    /// - `id` of the [Room].
    pub fn load_sync(id: &str) -> Result<Self, CgError> {
        Ok(serde_json::from_str(&sync_fs::read_to_string(room_fp(id))?)?)
    }

    /// Create a new room (or load one if corresponding file exists for `id`).
    /// 
    /// # Args
    /// - `id` of the new (or loaded) [Room].
    /// - *[[potential]]* `title` of the new [Room]. This is ignored if [Room] gets loaded.
    pub async fn new(id: &str, title: &str) -> Result<Arc<RwLock<Self>>, CgError> {
        // check if there is pre-existing file...
        let loaded = Room::load_sync(id);
        let room = match loaded {
            Ok(room) => room,
            _ => {
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
                memory_fog: None,
            }
            }
        };

        Ok(Arc::new(RwLock::new(room)))
    }

    /// Save the [Room].
    pub async fn save(&self) -> Result<(), CgError> {
        let path = room_fp(&self.id);
        log::debug!("Saving '{}'…", path.display());
        async_fs::write(path, serde_json::to_string_pretty(self)?).await?;
        Ok(())
    }

    /// Bootstrap phase exits linker.
    pub fn bootstrap_exits(&mut self, world: &World) {
        for (dir, exl) in self.raw_exits.drain() {
            let room_weak = if let Some(ref room_id) = exl.room_id {
                let Some(room_arc) = world.get_room_by_m_id(room_id.as_m_id()) else {
                    log::warn!("No target Room found: {dir} @ {exl:?}");
                    continue;
                };
                Arc::downgrade(&room_arc)
            } else {
                Weak::new()
            };
            if let Some(ref r_id) = exl.room_id {
                log::trace!("Linked {}:{dir} to {r_id:?}", self.id);
            } else {
                log::trace!("Mirage exit {}:{dir}", self.id);
            }
            self.exits.insert(dir.clone(), Exit::from(exl.clone(), room_weak));
        }
    }

    /// Assign an [Exit]. Existing [`dir`][Direction] will be overwritten.
    pub async fn assign_exit(&mut self, dir: Direction, exit: Exit) {
        self.raw_exits.insert(dir.clone(), ExitLike::from(&exit).await);
        self.exits.insert(dir, exit);
    }

    /// Eradicate exit at `dir` if exists.
    pub fn remove_exit(&mut self, dir: &Direction) {
        self.exits.remove(dir);
    }

    /// Generate a shallow clone of self.
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
            memory_fog: self.memory_fog.clone(),
        }
    }

    /// Extract specific internals of `source`.
    pub fn scavenge(&mut self, source: Self) {
        self.id = source.id;
        self.title = source.title;
        self.desc = source.desc;
        self.exits = source.exits;
        self.raw_exits = source.raw_exits;
        self.memory_fog = source.memory_fog;
    }

    /// Convenience function to try insert `item` directly into the [Room]'s contents.
    pub fn try_insert(&mut self, item: Item) -> Result<(), StorageError> {
        self.contents.try_insert(item)
    }

    /// List adjacent [rooms][Room], if any.
    pub fn list_adjacent(&self) -> Vec<Weak<RwLock<Room>>> {
        self.exits.iter().map(|(_,r)| r.as_weak()).collect::<Vec<Weak<RwLock<Room>>>>()
    }

    /// List adjacent [rooms][Room], if any, using BFS.
    /// Although this is very swift, it's not the most awesome of ideas to use too high `depth` value.
    pub async fn list_adjacent_bfs(start: &Arc<RwLock<Room>>, depth: u8) -> Vec<Weak<RwLock<Room>>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut nearby: HashMap<usize, Weak<RwLock<Room>>> = HashMap::new();

        let start_w = lock2key!(arc start);
        queue.push_back((Arc::downgrade(start), 0));
        visited.insert(start_w);
        while let Some((r_weak, dist)) = queue.pop_front() {
            if dist > depth { continue; }
            nearby.insert(lock2key!(weak &r_weak), r_weak.clone());
            if dist >= depth { continue; }// we skip current depth, we were the "end of line in" in this piece of queue.
            
            if let Some(r) = r_weak.upgrade() {
                let lock = r.read().await;
                for (_, ex) in &lock.exits {
                    let key = lock2key!(weak &ex.as_weak());
                    if !visited.contains(&key) {
                        visited.insert(key);
                        queue.push_back((ex.as_weak().clone(), dist + 1));
                    }
                }
            }
        }

        nearby.values().into_iter().map(|w| w.clone()).collect::<Vec<Weak<RwLock<Room>>>>()
    }

    /// Get the [Room]'s [memory fog][MemoryFog], if any.
    pub fn memory_fog(&self) -> Option<MemoryFog> {
        self.memory_fog.clone()
    }

    /// Check if we have an [Exit] at [`dir`][Direction].
    pub fn contains_exit(&self, dir: &Direction) -> bool {
        self.exits.contains_key(dir)
    }
}

impl<'a> IntoIterator for &'a Room {
    type Item = (&'a String, &'a Item);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.contents.into_iter()
    }
}

impl Storage for Room {
    fn can_hold(&self, item: &Item) -> Result<(), StorageQueryError> { self.contents.can_hold(item) }
    fn contains(&self, id: &str) -> bool { self.contents.contains(id) }
    fn eject_all(&mut self) -> Option<Vec<Item>> { None }
    fn find_id_by_name(&self, name: &str) -> Option<String> { self.contents.find_id_by_name(name) }
    fn max_space(&self) -> StorageSpace { self.contents.max_space() }
    fn peek_at(&self, id: &str) -> Option<&Item> { self.contents.peek_at(id) }
    fn peek_at_mut(&mut self, id: &str) -> Option<&mut Item> { self.contents.peek_at_mut(id) }
    fn required_space(&self) -> StorageSpace { StorageSpace::MAX }
    fn space(&self) -> StorageSpace { self.contents.space() }
    fn take(&mut self, id: &str) -> Option<Item> { self.contents.take(id) }
    fn take_by_name(&mut self, id: &str) -> Option<Item> { self.contents.take_by_name(id) }
    fn try_insert(&mut self, item: Item) -> Result<(), StorageError> { self.contents.try_insert(item) }
}

impl StorageMut for Room {
    fn set_max_space(&mut self, sz: StorageSpace) -> bool { self.contents.set_max_space(sz) }
}

impl Room {
    pub async fn tick(&mut self) {
        let max_par = crate::world::CPU_CORES;
        let sem = Arc::new(tokio::sync::Semaphore::new(max_par));
        let mut join_set = tokio::task::JoinSet::new();

        for p_weak in self.who.values() {
            if let Some(p_arc) = p_weak.upgrade() {
                if let Ok(mut p) = p_arc.try_write() {
                    // no reaction yet to "positive" tick(s)
                    let _= p.tick();
                }
            }
        }

        for e in self.entities.values() {
            let sem_clone = Arc::clone(&sem);
            let e_clone = e.clone();
            join_set.spawn(async move {
                let _permit = sem_clone.acquire_owned().await.unwrap();
                if let Ok(mut e) = e_clone.try_write() {
                    _ = e.tick().await;
                }
            });
        }

        // no reaction yet to "positive" tick(s)
        let _ = self.contents.tick();
        #[cfg(all(debug_assertions,feature = "stresstest"))]{
            log::debug!("Room '{}' ticked.", self.id);
        }
    }
}
