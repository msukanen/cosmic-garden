//! Roomies.

use std::{collections::HashMap, fmt::Display, fs as sync_fs, sync::{Arc, Weak}};

use cosmic_garden_pm::IdentityMut;
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, fs as async_fs};

use crate::{error::Error, identity::IdentityQuery, io::DATA_PATH, player::Player, string::Slugger, util::direction::Direction, world::World};

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

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut)]
pub struct Room {
    id: String,
    title: String,
    #[serde(default, skip)]
    pub who: HashMap<String, Weak<RwLock<Player>>>,

    #[serde(default, skip)]
    pub exits: HashMap<Direction, Weak<RwLock<Room>>>,
    
    #[serde(default)]
    pub raw_exits: HashMap<Direction, String>,
}

impl Room {
    pub fn load_sync(id: &str) -> Result<Self, Error> {
        let path = format!("{}/room/{id}.json", *DATA_PATH);
        log::debug!("Loading '{path}'…");
        let room: Room = serde_json::from_str(
            &sync_fs::read_to_string(path)?
        )?;
        Ok(room)
    }

    pub async fn new(id: &str, title: &str) -> Result<Arc<RwLock<Self>>, Error> {
        let room = Self {
            id: id.as_id()?,
            title: title.into(),
            who: HashMap::new(),
            exits: HashMap::new(),
            raw_exits: HashMap::new(),
        };
        room.save().await?;

        Ok(Arc::new(RwLock::new(room)))
    }

    pub async fn save(&self) -> Result<(), Error> {
        let path = format!("{}/room/{}.json", *DATA_PATH, self.id());
        log::debug!("Saving '{path}'…");
        async_fs::write(path, serde_json::to_string_pretty(self)?).await?;
        Ok(())
    }

    pub async fn link_exit(&mut self, world: Arc<RwLock<World>>, dir: Direction, target_id: &str) -> Result<(), Error> {
        // Exits are allowed to point to non-existing ways…
        log::debug!("Linking '{}'({dir}) to '{target_id}'…", self.id());
        self.raw_exits.insert(dir.clone(), target_id.into());
        // Find the target room, hopefully.
        let w = world.read().await;
        let my_lock = if let Some(my_arc) = w.rooms.get(self.id()) {
            Arc::downgrade(my_arc)
        } else {
            log::error!("Wait what? Where did '{}' lock go?!", self.id());
            return Err(Error::from(RoomError::NoSuchRoom(self.id().to_string())))
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

        Err(Error::from(RoomError::NoSuchRoom(target_id.into())))
    }
}

#[cfg(test)]
mod room_tests {
    use std::sync::Arc;

    use tokio::sync::RwLock;

    use crate::{Cli, DATA, io::DATA_PATH, util::direction::Direction, world::{self, World}};

    #[tokio::test]
    async fn world_room_linking() {
        let _ = env_logger::try_init();
        let _ = DATA.set(std::env::var("COSMIC_GARDEN_DATA").unwrap());
        let mut args = Cli {
            autosave_queue_interval: None,
            host_listen_addr: "0.0.0.0".into(),
            host_listen_port: 8080,
            world: "cosmic-garden".into(),
            data_path: (*DATA_PATH).clone(),
            bootstrap_url: None,
        };
        let w = World::load_or_bootstrap(&args).await.unwrap_or_else(|e| panic!("Oh noes! Not the dreaded {e:?}"));
        w.link_rooms().await;
        let rooms = w.rooms.clone();
        let w_arc = Arc::new(RwLock::new(w));
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
    }
}
