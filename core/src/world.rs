//! When worlds collide…
use std::{collections::HashMap, net::SocketAddr, sync::Arc, usize};

use futures::{StreamExt, stream};
use nohash_hasher::BuildNoHashHasher;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::{RwLock, Semaphore}, task::JoinSet};

use crate::{Cli, error::CgError, identity::{IdError, IdentityQuery, MachineId, MachineIdentity, uniq::UuidValidator}, io::world_fp, item::Item, mob::EntityWeak, player::{Player, PlayerArc}, room::{Room, RoomArc, locking::Exit}, string::{UNNAMED, prompt::PromptType}, thread::{SystemSignal, signal::SignalSenderChannels}, util::direction::Direction};

const NUM_ROOMS_FOR_PARALLEL_SHIFT: usize = 50;
const NUM_WORLD_IDENT_ROOMS_IN_PARALLEL: usize = 50;
pub(crate) const CPU_CORES: usize = 16;// adjust to whatever number of cores your server has…

/// The world!
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct World {
    /// World's printable name.
    pub name: String,
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
    pub players_by_sockaddr: HashMap<SocketAddr, PlayerArc>,
    /// Players sorted by user's login ID (not their [Player] character's ID).
    #[serde(skip, default)]
    pub players_by_id: HashMap<String, PlayerArc>,

    #[serde(rename = "rooms", with = "room_id_sieve")]
    rooms: HashMap<MachineId, RoomArc, BuildNoHashHasher<MachineId>>,
    #[serde(default = "default_root_room_id")]
    pub root_room_id: String,
    #[serde(skip)]
    pub root_room: Option<RoomArc>,
    #[serde(default, skip)]
    pub entities: HashMap<MachineId, EntityWeak, BuildNoHashHasher<MachineId>>,

    #[serde(default)]
    pub lost_and_found: HashMap<MachineId, Item>,
    #[serde(skip)]
    pub channels: Option<SignalSenderChannels>,
}
/// World arc type.
pub type WorldArc = Arc<RwLock<World>>;

impl World {
    #[cfg(test)]
    pub async fn dummy() -> Self {
        let root_room = Some(Room::new("r-1", "Incineration Chamber").await.unwrap());
        let room_2 = Some(Room::new("r-2", "Waterfall").await.unwrap());
        Self {
            name: "Test World".into(),
            port: 8080,
            greeting: "Greetings, Crash Test Dummy!".to_string().into(),
            fixed_prompts: HashMap::new(),
            players_by_sockaddr: HashMap::new(),
            players_by_id: HashMap::new(),
            root_room_id: "r-1".into(),
            rooms: {
                use crate::identity::MachineIdentity;
                let mut m = HashMap::default();
                m.insert("r-1".as_m_id(), root_room.clone().unwrap());
                m.insert("r-2".as_m_id(), room_2.clone().unwrap());
                m},
                root_room,
            lost_and_found: HashMap::new(),
            channels: None,
            entities: HashMap::default(),
        }
    }
}

mod room_id_sieve {
    use super::*;
    use serde::{Serializer, Deserializer, Deserialize, ser::SerializeSeq};

    /// HashMap → list of IDs.
    pub fn serialize<S>(rooms: &HashMap<MachineId, RoomArc, BuildNoHashHasher<MachineId>>, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut seq = s.serialize_seq(Some(rooms.len()))?;
        for id in rooms.keys() {
            seq.serialize_element(id)?;
        }
        seq.end()
    }

    /// Take the list of IDs and materialize the [Room]s from disk.
    pub fn deserialize<'de, D>(d: D) -> Result<HashMap<MachineId, RoomArc, BuildNoHashHasher<MachineId>>, D::Error>
    where D: Deserializer<'de> {
        let ids = Vec::<String>::deserialize(d)?;
        let mut rooms = HashMap::default();

        for id in ids {
            // Using a blocking load because this happens during [World::load_or_bootstrap].
            let room = Room::load_sync(&id).map_err(serde::de::Error::custom)?;
            rooms.insert(id.as_m_id(), Arc::new(RwLock::new(room)));
        }
        Ok(rooms)
    }
}

fn default_root_room_id() -> String {
    "room-1".into()
}

impl World {
    /// Load or bootstrap the world.
    pub async fn load_or_bootstrap(args: &Cli) -> Result<Self, CgError> {
        match fs::read_to_string(world_fp()).await {
            Ok(content) => {
                let world: World = serde_json::from_str( &content )?;
                Ok(world)
            },
            Err(_) => {
                // bootstrapping required. No world found.
                let w = Self {
                    port: args.host_listen_port,
                    greeting: format!("Welcome to {}!", args.world).into(),
                    name: args.world.clone(),
                    fixed_prompts: HashMap::new(),
                    players_by_id: HashMap::new(),
                    players_by_sockaddr: HashMap::new(),
                    root_room_id: default_root_room_id(),
                    root_room: None,
                    rooms: {
                        let mut rooms = HashMap::default();
                        
                        let r1 = Room::new(default_root_room_id().as_str(), "Room #1").await?;
                        let r1_id = r1.read().await.id().to_string();
                        rooms.insert(r1_id.as_m_id(), r1.clone());

                        let r2 = Room::new("room 2!", "Room #2").await?;
                        let r2_id = r2.read().await.id().to_string();
                        rooms.insert(r2_id.as_m_id(), r2.clone());                       
                        {
                            let mut l1 = r1.write().await;
                            let exit = Exit::Free { room: Arc::downgrade(&r2) };
                            l1.assign_exit(Direction::North, exit).await;
                            l1.save().await?;
                        }
                        {   
                            let mut l2 = r2.write().await;
                            let exit = Exit::Free { room: Arc::downgrade(&r1) };
                            l2.assign_exit(Direction::South, exit).await;
                            l2.save().await?;
                        }
                        rooms
                    },
                    lost_and_found: HashMap::new(),
                    channels: None,
                    entities: HashMap::default(),
                };
                w.save(true).await?;
                log::info!("Brand new world: '{}'; bootstrapped successfully.", w.name);
                Ok(w)
            }
        }
    }

    /// Save the world! Yeah.
    pub async fn save(&self, force_save: bool) -> Result<(), CgError> {
        let w = self.clone();
        tokio::spawn(async move {
            if !w.lost_and_found.is_empty() || force_save {
                let contents = serde_json::to_string_pretty(&w).expect("ERROR WITH World JSON!");
                fs::write(world_fp(), contents).await.expect("OS BEING A B****!");
                log::info!("World '{}' saved.", w.name);
            }
        });
        Ok(())
    }

    /// Insert [Player] to mappings.
    /// 
    /// # Args
    /// - [`world`][World] arc.
    /// - `addr`; IPv4/IPv6
    /// - `id` of the [Player].
    /// - `arc` of the [Player].
    pub async fn insert_player(world: WorldArc, addr: &SocketAddr, id: &str, arc: PlayerArc) {
        {// map the soul…
            let mut w = world.write().await;
            w.players_by_sockaddr.insert(addr.clone(), arc.clone());
            if let Some(old_arc) = w.players_by_id.get(id) {
                w.channels.as_ref().and_then(|out|out.janitor.send(SystemSignal::PlayerLogout { player: old_arc.clone() }).ok());
            }
            w.players_by_id.insert(id.into(), arc.clone());
        }

        let (root_id, current_loc, p_id) = {
            let w = world.read().await;
            let p = arc.read().await;
            (w.root_room_id.clone(), p.location_id.clone(), p.id().to_string())
        };

        log::trace!("Some soul transplanting…");
        let target_id = if current_loc == UNNAMED { &root_id} else { &current_loc };
        let room_to_place_in = {
            let w = world.read().await;
            w.rooms.get(&target_id.as_m_id()).cloned()
        };
        let Some(room) = room_to_place_in else {
            log::warn!("Cannot place '{p_id}' where they wanted to go; room '{target_id}' does not exist.");
            log::debug!("Translocating '{p_id}' to root.");
            let target_arc = world.read().await.root_room.clone().expect("Root room evaporated?!");
            if let Err(e) = Player::place_direct(arc, target_arc).await {
                log::error!("Facepalming here; {e:?}");
            }
            return
        };
        log::trace!("Acquired room '{}'…", room.read().await.id());
        log::debug!("Bracing for place_direct()…");
        if let Err(e) = Player::place_direct(arc, room.clone()).await {
            log::error!("Facepalming here; {e:?}");
            return;
        }
        log::trace!("Done translocating…");
    }

    /// Establish links between [Room(s)][Room].
    pub async fn link_rooms(&mut self) {
        let rooms = self.rooms.clone();
        for room_arc in rooms.values() {
            let mut room = room_arc.write().await;
            room.bootstrap_exits(&self);
            log::trace!("Room id '{}' vs sought for '{}'…", room.id(), self.root_room_id);
            // weld the root room in place
            if room.id() == self.root_room_id {
                log::trace!("Welding '{}' as root room.", room.id());
                self.root_room = room_arc.clone().into();
            }
        }

        if self.root_room.is_none() {
            let msg = format!("FATAL: root room not found! Attempted '{}'… no match.", self.root_room_id);
            log::error!("{msg}");
            panic!("{msg}");
        };

        log::info!("Root room established as '{}'", self.root_room_id)
    }

    /// Try get `Arc` of a [Room] by `id`…
    pub fn get_room_by_id(&self, id: &str) -> Option<RoomArc> {
        self.get_room_by_m_id(id.as_m_id())
    }

    /// Try get `Arc` of a [Room] by [machine ID][MachineId].
    pub fn get_room_by_m_id(&self, m_id: MachineId) -> Option<RoomArc> {
        self.rooms.get(&m_id).cloned()
    }

    /// Try insert a new [Room].
    /// 
    /// # Args
    /// - [`room`][Room] to insert.
    /// # Returns
    /// `Ok` or [IdError].
    pub async fn insert_room(&mut self, room: RoomArc) -> Result<(), IdError> {
        let id = room.read().await.id().to_string();
        let m_id = id.as_m_id();
        self.insert_room_by_m_id(m_id, id, room)
    }

    /// Try insert a new [Room]. Seldom used directly…
    /// 
    /// For the sake of sanity, direct overwrite is not allowed.
    /// 
    /// # Args
    /// - `m_id` of the [Room].
    /// - `id` of the [Room].
    /// - [`room`][Room] to insert.
    /// 
    /// # Returns
    /// `Ok` or [IdError].
    pub fn insert_room_by_m_id(&mut self, m_id: MachineId, id: String, room: RoomArc) -> Result<(), IdError> {
        if self.rooms.contains_key(&m_id) {
            log::error!("Builder: Room ID '{id}' collision course detected. Refusing banging them against each other.");
            return Err(IdError::ReservedName(id));
        }

        log::trace!("Room '{id}'({m_id}) registered");
        self.rooms.insert(m_id, room);
        Ok(())
    }
}

/// Room pagination result type.
pub struct RoomPaginateResult {
    /// ID/Arc
    pub entries: Vec<(MachineId, RoomArc)>,
    /// Total num of search matches.
    pub total_found: usize,
    /// Total pages (relative to `per_page` of the search).
    pub total_pages: usize,
}

impl World {
    pub async fn tick(&mut self) {
        let max_par = CPU_CORES;
        let sem = Arc::new(Semaphore::new(max_par));
        let mut join_set = JoinSet::new();

        for r in self.rooms.values() {
            let sem_clone = Arc::clone(&sem);
            let r_clone = r.clone();
            join_set.spawn(async move {
                let _permit = sem_clone.acquire_owned().await.unwrap();
                let mut r = r_clone.write().await;
                r.tick().await;
            });
        }
    }

    /// Get room entries as per id/title needle search in "pages" (see [RoomPaginateResult]).
    pub async fn paginated_room_entries(&self, needle: &str, page: usize, mut per_page: usize) -> RoomPaginateResult {
        if per_page == 0 {
            per_page = usize::MAX;
        }
        let needle = needle.to_lowercase();
        let id_needle = needle.as_id().unwrap_or("**garbage**".into());
        let mut pages = if self.rooms.len() < NUM_ROOMS_FOR_PARALLEL_SHIFT {
            let mut res = Vec::new();
            for (id, r) in self.rooms.iter() {
                let lock = r.read().await;
                let title = lock.title().to_lowercase();
                log::debug!("{id} @ {title}");
                if lock.id().contains(&id_needle) || title.contains(&needle) {
                    res.push((*id, r.clone()));
                }
            }
            res
        } else {
            let room_clones: Vec<(MachineId,RoomArc)> = self.rooms.iter().map(|(id,r)|(id.clone(),r.clone())).collect();
            stream::iter(room_clones)
                .map(|(id,r)| {
                    let id_needle = id_needle.clone();
                    let needle = needle.clone();
                    let id = id.clone();
                    let r = r.clone();
                    async move {
                        let lock = r.read().await;
                        let title = lock.title().to_lowercase();
                        let matches = lock.id().contains(&id_needle) || title.contains(&needle);
                        (id,r.clone(),matches)
                    }
                })
                .buffered(NUM_WORLD_IDENT_ROOMS_IN_PARALLEL) // TODO adjust?
                .filter(|(_,_,matches)| futures::future::ready(*matches))
                .map(|(id,r,_)| (id,r))
                .collect::<Vec<_>>()
                .await
        };
        pages.sort_unstable_by(|a,b| a.0.cmp(&b.0));
        let total_found = pages.len();
        let entries = pages.into_iter().skip(page.saturating_sub(1) * per_page).take(per_page).collect();
        RoomPaginateResult {
            entries, total_found, total_pages: (total_found + per_page - 1) / per_page // saturating_add would be safer, but anywhere close to usize::MAX rooms, really?
        }
    }

    fn room_list(&self, term: Option<String>, out: tokio::sync::oneshot::Sender<Vec<MachineId>>) {
        let list: Vec<MachineId> = self.rooms.keys().cloned().collect();
        tokio::spawn(async move { (list, term, out) });
    }
}

pub async fn room_list(world: WorldArc, term: Option<String>) -> Vec<MachineId> {
    let (out, recv) = tokio::sync::oneshot::channel::<Vec<MachineId>>();
    world.read().await.room_list(term, out);
    recv.await.unwrap_or_default()
}

#[cfg(test)]
pub(crate) mod world_tests {
    use std::sync::Once;

    use crate::{cformat, identity::{IdentityMut, MachineIdentity}, player::PlayerArc, world::WorldArc};

    pub static DISK_VERIFIED: Once = Once::new();

    pub(crate) async fn get_operational_mock_world() -> (
        WorldArc,
        crate::SignalChannels,
        (   crate::io::ClientState,
            PlayerArc
        ),
        (   tokio::sync::oneshot::Sender<()>,
            tokio::sync::oneshot::Receiver<()>
        ),
    ){
        use std::io::Write;
        let _ = env_logger::
             Builder::from_default_env()
             .format(|buf, record| {
                 let chunk = record.args().to_string();
                 let mut msg = chunk.split('\n');
                 if let Some(x) = msg.next() {
                     writeln!(buf, "{}", cformat!("<c white>[<c yellow>{}</c> <c cyan>{}</c>]</c> {x}", record.level(), record.module_path().unwrap_or_default()))?;
                 }

                 for line in msg {
                     writeln!(buf, "{}", cformat!("<bg gray>    </bg>{}", line))?;
                 }

                 Ok(())
                
             }).
            try_init();
        DISK_VERIFIED.call_once(|| {
            let _ = crate::DATA.get_or_init(|| "data".into());
            let _ = crate::WORLD.get_or_init(|| "crash-test-dummy".to_string());
            let _ = crate::thread::life::CORE_HZ.get_or_init(|| 100);
            let _ = crate::thread::life::BATTLE_HZ.get_or_init(|| 50);
            let path = std::path::Path::new("data/crash-test-dummy");
            if path.exists() {
                log::trace!("Persistence verified.");
            } else {
                log::info!("Bootstrap missing.");
            }
        });
        use crate::identity::IdentityQuery;
        // world basics…
        let mut world = crate::world::World::dummy().await;
        
        // create Player#1
        let mut plr = crate::player::Player::default();
        plr.set_id("test-player-1", true).ok();
        let plr_id = plr.id().to_string();
        let plr = std::sync::Arc::new(tokio::sync::RwLock::new(plr));
        world.players_by_id.insert(plr_id.clone(), plr.clone());
        // put player#1 into r-1
        let Some(r) = world.rooms.get(&"r-1".as_m_id()) else { panic!("r-1 missing?!")};
        r.write().await.who.insert(plr_id.clone(), std::sync::Arc::downgrade(&plr));
        plr.write().await.location = std::sync::Arc::downgrade(&r);

        // signal channels…
        let sigs = crate::SignalChannels::default();
        world.channels = sigs.out.clone().into();

        let (dtx,drx) = tokio::sync::oneshot::channel::<()>();

        (   std::sync::Arc::new(tokio::sync::RwLock::new(world)),
            sigs,
            (crate::io::ClientState::Playing { player: plr.clone() }, plr.clone()),
            (dtx, drx),
        )
    }
}
