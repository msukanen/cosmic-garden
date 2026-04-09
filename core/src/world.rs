//! When worlds collide…
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::{Cli, error::Error, identity::IdentityQuery, io::DATA_PATH, player::Player, room::Room, string::{Slugger, prompt::PromptType}};

/// The world!
#[derive(Debug, Deserialize, Serialize)]
pub struct World {
    /// World's printable name.
    pub name: String,
    /// Location of the .world file itself.
    #[serde(skip)]
    path: PathBuf,
    /// Port# the world listens on.
    pub port: u16,

    /// Optional greeting message override.
    #[serde(default)]
    pub greeting: Option<String>,
    /// Optional prompt overrides.
    #[serde(default)]
    pub fixed_prompts: HashMap<PromptType, String>,

    /// Players sorted by their direct socket address.
    #[serde(skip, default)]
    pub players_by_sockaddr: HashMap<SocketAddr, Arc<RwLock<Player>>>,
    /// Players sorted by user's login ID (not their [Player] character's ID).
    #[serde(skip, default)]
    pub players_by_id: HashMap<String, Arc<RwLock<Player>>>,

    #[serde(rename = "rooms", with = "room_id_sieve")]
    pub rooms: HashMap<String, Arc<RwLock<Room>>>,
}

mod room_id_sieve {
    use super::*;
    use serde::{Serializer, Deserializer, Deserialize, ser::SerializeSeq};

    /// HashMap → list of IDs.
    pub fn serialize<S>(rooms: &HashMap<String, Arc<RwLock<Room>>>, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut seq = s.serialize_seq(Some(rooms.len()))?;
        for id in rooms.keys() {
            seq.serialize_element(id)?;
        }
        seq.end()
    }

    /// Take the list of IDs and materialize the [Room]s from disk.
    pub fn deserialize<'de, D>(d: D) -> Result<HashMap<String, Arc<RwLock<Room>>>, D::Error>
    where D: Deserializer<'de> {
        let ids = Vec::<String>::deserialize(d)?;
        let mut rooms = HashMap::new();

        for id in ids {
            // Using a blocking load because this happens during [World::load_or_bootstrap].
            let room = Room::load_sync(&id).map_err(serde::de::Error::custom)?;
            rooms.insert(id, Arc::new(RwLock::new(room)));
        }
        Ok(rooms)
    }
}

impl World {
    /// Load or bootstrap the world.
    pub async fn load_or_bootstrap(args: &Cli) -> Result<Self, Error> {
        let path = PathBuf::from(format!("{}/{}.world", *DATA_PATH, args.world.as_id()?));
        match fs::read_to_string(&path).await {
            Ok(content) => {
                let mut world: World = serde_json::from_str( &content )?;
                world.path = path;
                Ok(world)
            },
            Err(_) => {
                // bootstrapping required. No world found.
                let w = Self {
                    path: path,
                    port: args.host_listen_port,
                    greeting: format!("Welcome to {}!", args.world).into(),
                    name: args.world.clone(),
                    fixed_prompts: HashMap::new(),
                    players_by_id: HashMap::new(),
                    players_by_sockaddr: HashMap::new(),
                    rooms: {
                        let mut rooms = HashMap::new();
                        let room = Room::new("room 1", "Room #1").await?;
                        rooms.insert(room.read().await.id().into(), room.clone());
                        let room = Room::new("room 2!", "Room #2").await?;
                        rooms.insert(room.read().await.id().into(), room.clone());
                        rooms
                    }
                };
                w.save().await?;
                Ok(w)
            }
        }
    }

    /// Save the world! Yeah.
    pub async fn save(&self) -> Result<(), Error> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&self.path, contents).await?;
        log::info!("World '{}' bootstrapped onto disk.", self.name);
        Ok(())
    }

    /// Insert [Player] to mappings.
    pub async fn insert_player(world: Arc<RwLock<World>>, addr: &SocketAddr, id: &str, arc: Arc<RwLock<Player>>) {
        let mut w = world.write().await;
        w.players_by_sockaddr.insert(addr.clone(), arc.clone());
        w.players_by_id.insert(id.into(), arc.clone());
    }

    pub async fn link_rooms(&self) {
        let rooms = self.rooms.clone();
        for room_arc in rooms.values() {
            let mut room = room_arc.write().await;
            let mut linked = HashMap::new();
            for (dir, target_id) in &room.raw_exits {
                if let Some(target_arc) = self.rooms.get(target_id) {
                    linked.insert(dir.clone(), Arc::downgrade(target_arc));
                } else {
                    log::warn!("Broken bridge… {} -> {} ({})", room.id(), target_id, dir);
                }
            }
            room.exits = linked;
        }
    }
}
