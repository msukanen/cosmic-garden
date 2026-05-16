//! Roomies.

use std::{collections::{HashMap, HashSet, VecDeque}, fmt::Display, fs as sync_fs, sync::{Arc, Weak}};

use cosmic_garden_pm::{DescribableMut, IdentityMut};
use lazy_static::lazy_static;
use nohash_hasher::BuildNoHashHasher;
use serde::{Deserialize, Serialize};
use tokio::{fs as async_fs, sync::{RwLock, Semaphore}};

use crate::{r#const::CPU_CORES, error::CgError, identity::{IdentityQuery, MachineId, MachineIdentity, uniq::{StrUuid, UuidValidator}}, io::{Broadcast, room_fp}, item::{Item, container::{storage::{Storage, StorageError, StorageMut, StorageQueryError, StorageSpace}, variants::{ContainerVariant, ContainerVariantType}}}, mob::{EntityArc, ai::AiAction}, player::PlayerWeak, room::{environ::{GRAVITY_ANOMALY_HIGH_H, GRAVITY_ANOMALY_LOW_H, MemoryFogType, SPECIAL_ENVIRONMENT_DEFAULT, SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY, SpecialEnvironment, SpecialEnvironmentError, Terrain}, locking::{Exit, ExitState}}, string::slug::Slugger, traits::{TickMeaning, Tickable}, util::direction::Direction, world::World};

pub mod environ;
pub mod locking;
pub mod types;      pub use types::{ RoomType, RoomSubtype };

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

/// Payload for [SystemSignal].
pub enum RoomPayload {
    Id(String),
    Arc(RoomArc)
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

impl From<RoomArc> for RoomPayload {
    fn from(value: RoomArc) -> Self {
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
fn room_sem_default() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(CPU_CORES))
}

type DirectionHasher = std::collections::hash_map::RandomState;

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, DescribableMut)]
pub struct Room {
    id: String, #[serde(skip)] pub(super) m_id: MachineId,
    title: String,
    #[serde(default = "empty_room_desc")]
    pub desc: String,
    #[serde(default, skip)]
    pub who: HashMap<String, PlayerWeak>,

    #[serde(default, skip)]
    pub exits: HashMap<Direction, Exit, DirectionHasher>,
    
    #[serde(default)]
    raw_exits: HashMap<Direction, ExitLike, DirectionHasher>,

    #[serde(default = "room_inventory")]
    contents: ContainerVariant,

    /// NPC [entities][Entity] in the [Room].
    // [Room] is the sole owner of an [Entity].
    #[serde(default, with = "arc_n_t_transform")]
    entities: HashMap<MachineId, EntityArc, BuildNoHashHasher<MachineId>>,

    /// Special environment bitmask.
    #[serde(default)] pub special_environment: SpecialEnvironment,
    #[serde(default)] memory_fog: Option<MemoryFogType>,
    /// Terrain, if not "typical"(≡`None`).
    #[serde(default)] pub terrain: Option<Terrain>,
    /// Room's general type.
    #[serde(default)] pub room_type: RoomType,
    #[serde(skip, default = "room_sem_default")] sem: Arc<Semaphore>,
    #[serde(skip, default = "mock_broadcast")] pub(super) out: tokio::sync::broadcast::Sender<Broadcast>,
}
/// Room arc type.
pub type RoomArc = Arc<RwLock<Room>>;
impl Into<RoomArc> for Room {
    fn into(self) -> RoomArc {
        std::sync::Arc::new(tokio::sync::RwLock::new(self))
    }
}
/// Room weak arc type.
pub type RoomWeak = Weak<RwLock<Room>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExitLike {
    #[serde(default)]
    pub room_id: Option<String>,
    #[serde(default)]
    pub key_bp: Option<String>,
    pub state: ExitState,
}

impl ExitLike {
    pub async fn from(ex_id: Option<String>, ex: &Exit) -> Self {
        match ex {
            Exit::Closed { key_bp,..} =>
                Self {
                    room_id: ex_id,
                    key_bp: key_bp.clone(),
                    state: ExitState::Closed
                },
            Exit::Free {..} =>
                Self {
                    room_id: ex_id,
                    key_bp: None,
                    state: ExitState::Free
                },
            Exit::Locked { key_bp,..} =>
                Self {
                    room_id: ex_id,
                    key_bp: key_bp.clone().into(),
                    state: ExitState::Locked
                },
            Exit::LockedAL { key_bp,..} =>
                Self {
                    room_id: ex_id,
                    key_bp: key_bp.clone().into(),
                    state: ExitState::LockedAL
                },
            Exit::Open { key_bp,..} =>
                Self {
                    room_id: ex_id,
                    key_bp: key_bp.clone(),
                    state: ExitState::Open
                },
            Exit::OpenAL { key_bp,..} =>
                Self {
                    room_id: ex_id,
                    key_bp: key_bp.clone().into(),
                    state: ExitState::OpenAL
                },
        }
    }
}

mod arc_n_t_transform {
    use std::{collections::HashMap, sync::Arc};

    use nohash_hasher::BuildNoHashHasher;
use serde::{Deserialize, Deserializer, Serializer, ser::SerializeMap};
    use tokio::sync::RwLock;

    use crate::{identity::MachineId, mob::{EntityArc, core::Entity}};

    pub fn serialize<S>(what: &HashMap<MachineId, EntityArc, BuildNoHashHasher<MachineId>>, s:S) -> Result<S::Ok, S::Error>
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

    pub fn deserialize<'de, D>(d: D) -> Result< HashMap<MachineId, EntityArc, BuildNoHashHasher<MachineId>>, D::Error>
    where D: Deserializer<'de>
    {
        let raw: HashMap<MachineId, Entity> = HashMap::deserialize(d)?;
        let mut arced: HashMap<MachineId, Arc<RwLock<Entity>>, BuildNoHashHasher<MachineId>> = HashMap::with_capacity_and_hasher(
            raw.len(),
            BuildNoHashHasher::default()
        );
        for (id, ent) in raw {
            arced.insert(id, Arc::new(RwLock::new(ent)));
        }

        Ok(arced)
    }
}

fn mock_broadcast() -> tokio::sync::broadcast::Sender<Broadcast> {
    let (tx,_) = tokio::sync::broadcast::channel(1);
    tx
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
    pub async fn new(id: &str, title: &str, bootstrap: bool) -> Result<RoomArc, CgError> {
        // check if there is pre-existing file...
        let loaded = Room::load_sync(id);
        let room = match loaded {
            Ok(room) => room,
            _ => {
                log::info!("No archælogy possible, thus creating new room '{}'", id);
                let id = if bootstrap { id.slug()? } else { id.as_id()? };
                Self {
                    m_id: id.as_m_id(),
                    id,
                    title: title.into(),
                    desc: empty_room_desc(),
                    who: HashMap::new(),
                    exits: HashMap::default(),
                    raw_exits: HashMap::default(),
                    contents: room_inventory(),
                    entities: HashMap::default(),
                    special_environment: SPECIAL_ENVIRONMENT_DEFAULT,
                    memory_fog: None,
                    terrain: None,
                    room_type: RoomType::default(),
                    sem: room_sem_default(),
                    out: mock_broadcast(),
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
    /// 
    /// # Args
    /// - `dir`ection.
    /// - `exit_id` of the [Exit].
    /// - `exit` itself.
    //
    // `exit_id` has to be determined by the caller or we face potential deadlock(s).
    //
    pub async fn assign_exit(&mut self, dir: Direction, exit_id: Option<String>, exit: Exit) {
        self.raw_exits.insert(dir.clone(), ExitLike::from(exit_id, &exit).await);
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
            m_id: self.m_id,
            title: self.title.clone(),
            desc: self.desc.clone(),
            exits: self.exits.clone(),
            raw_exits: self.raw_exits.clone(),
            // we skip everything else:
            who: HashMap::new(),
            contents: room_inventory(),
            entities: HashMap::default(),
            special_environment: self.special_environment,
            memory_fog: self.memory_fog.clone(),
            terrain: self.terrain.clone(),
            room_type: self.room_type.clone(),
            sem: self.sem.clone(),
            out: self.out.clone(),
        }
    }

    /// Extract specific internals of `source`.
    pub async fn scavenge(&mut self, source: Self, world: &Arc<RwLock<World>>) {
        self.id = source.id;
        self.title = source.title;
        self.desc = source.desc;
        self.raw_exits = source.raw_exits;
        let mut exits = HashMap::default();
        let wr = world.read().await;
        for (dir, exitlike) in self.raw_exits.clone() {
            if let Some(t_id) = exitlike.room_id.clone() {
                if let Some(t_arc) = wr.get_room_by_id(&t_id) {
                    // proper room at receiving end
                    let exit = Exit::from_arc(exitlike, t_arc);
                    exits.insert(dir, exit);
                } else {
                    // old target room evaporated meanwhile …
                    exits.insert(dir, Exit::from(exitlike, Weak::new()));
                }
            } else {
                // mirage
                exits.insert(dir, Exit::from(exitlike, Weak::new()));
            }
        }
        drop(wr);
        self.exits = exits;
        self.special_environment = source.special_environment;
        self.memory_fog = source.memory_fog;
        self.terrain = source.terrain;
        self.room_type = source.room_type;
    }

    /// Convenience function to try insert `item` directly into the [Room]'s contents.
    pub fn try_insert(&mut self, item: Item) -> Result<(), StorageError> {
        self.contents.try_insert(item)
    }

    /// List adjacent [rooms][Room], if any.
    pub fn list_adjacent(&self) -> Vec<RoomWeak> {
        self.exits.iter().map(|(_,r)| r.as_weak()).collect::<Vec<RoomWeak>>()
    }

    /// List adjacent [rooms][Room], if any, using BFS.
    /// Although this is very swift, it's not the most awesome of ideas to use too high `depth` value.
    pub async fn list_adjacent_bfs(start: &RoomArc, depth: u8) -> Vec<RoomWeak> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut nearby: HashMap<usize, RoomWeak> = HashMap::new();

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
    /// 
    // NOTE: Although one of the many special environments,
    //       MemoryFog "needs" a bit different treatment when
    //       dealing with e.g. city jail exits.
    pub fn memory_fog(&self) -> Option<MemoryFogType> {
        self.memory_fog.clone()
    }

    /// Check if we have an [Exit] at [`dir`][Direction].
    pub fn contains_exit(&self, dir: &Direction) -> bool {
        self.exits.contains_key(dir)
    }

    /// Get special env bitmask.
    pub fn special_env_bitmask(&self) -> SpecialEnvironment {
        self.special_environment
    }

    /// Set special env bit`mask`.
    /// 
    /// # Args
    /// - `mask` to set.
    /// - `override` old setting(s) in entirety?
    #[must_use = "Gravity anomalies may result in `Err`."]
    pub fn set_special_env_bitmask(&mut self, mask: SpecialEnvironment, r#override: bool) -> Result<(), SpecialEnvironmentError> {
        match ((mask | self.special_environment) & (GRAVITY_ANOMALY_HIGH_H|GRAVITY_ANOMALY_LOW_H|SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY)).count_ones() {
            0 => (), // normal g
            1 => return Err(SpecialEnvironmentError::GravityModelMissing),
            2 => if mask & SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY == 0 { return Err(SpecialEnvironmentError::GravityClash) },
            _ => return Err(SpecialEnvironmentError::GravityClash)
        }

        if r#override {
            self.special_environment = mask;
        } else {
            self.special_environment |= mask;
        }
        Ok(())
    }

    /// Wipe environmental bitmask.
    #[inline]
    pub fn clear_env_bitmask(&mut self) {
        self.special_environment = SPECIAL_ENVIRONMENT_DEFAULT;
    }

    /// Remove given [Entity] from the [Room].
    #[inline]
    pub fn remove_entity(&mut self, id: MachineId) {
        self.entities.remove(&id);
    }

    /// Add [crate::mob::core::Entity][Entity] to [Room].
    #[inline]
    pub fn add_entity(&mut self, id: MachineId, ent: EntityArc) {
        self.entities.insert(id, ent);
    }

    /// Find [Entity] by [MachineId].
    #[inline]
    pub fn get_entity_by_m_id(&self, id: MachineId) -> Option<EntityArc> {
        self.entities.get(&id).cloned()
    }

    /// Get [crate::mob::Entity][Entity] by (one or the other) ID.
    pub async fn get_entity_by_id<Id: MachineIdentity + Display + UuidValidator>(&self, id: Id) -> Option<EntityArc> {
        if let Some(e) = self.get_entity_by_m_id(id.as_m_id()) {
            return e.into();
        }

        let (show_id, needle) = {
            let show_id = id.has_uuid();
            (show_id, id.to_string())
        };

        for ent in self.entities.values() {
            if ent.read().await.id().show_uuid(show_id).starts_with(&needle) {
                return ent.clone().into();
            }
        }

        None
    }

    /// Get count of [Entity] in the [Room].
    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Drain the entities for processing somewhere else.
    pub fn drain_entities(&mut self) -> Vec<(MachineId, EntityArc)>{
        self.entities.drain().collect()
    }

    /// Get the [Entity]s as an iterator.
    #[inline]
    pub fn entities(&self) -> impl Iterator<Item = (&MachineId, &EntityArc)> {
        self.entities.iter()
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

const fn bucket_scaler(num_things: usize) -> usize {
    const Y0: usize = 32;
    const X0: usize = 32;
    const Y1: usize = 1_000_000;
    const X1: usize = 5_000 * CPU_CORES / 16;
    if num_things <= Y0 { X0 }
    else {
        const DY: usize = Y1 - Y0;
        X0 + ((X1 - X0) * (num_things - Y0)) / DY
    }
}

impl Room {
    /// Tick the [Room].
    /// 
    /// By default we try to tick at 1/10th of the main core speed.
    pub async fn tick(&mut self, curr_tick: usize, room: RoomArc) {
        if self.m_id.wrapping_add(curr_tick) % 10 != 0 { return ;}
        
        // Deal with players first…
        for p_weak in self.who.values() {
            if let Some(p_arc) = p_weak.upgrade() {
                if let Ok(mut p) = p_arc.try_write() {
                    _ = p.tick(curr_tick, self.special_environment, self.terrain);
                }
            }
        }

        // …then entitites…
        // #[cfg(not(feature = "stresstest"))]
        // const BATCH_SIZE: usize = 10;
        // #[cfg(feature = "stresstest")]
        // const BATCH_SIZE: usize = 5_000;
        let batch_size: usize = bucket_scaler(self.entities.len());
        #[cfg(feature = "stresstest")]
        static mut AISC: usize = 0;
        let mut join_set = tokio::task::JoinSet::new();
        let mut curr_ent_batch = Vec::with_capacity(batch_size);
        for (_, e) in &self.entities {
            curr_ent_batch.push(e.clone());
            if curr_ent_batch.len() == batch_size {
                let batch = std::mem::take(&mut curr_ent_batch);
                let sem_clone = Arc::clone(&self.sem);
                let r_env = self.special_environment;
                let r_ter = self.terrain.clone();
                let r_tick = curr_tick;
                join_set.spawn(async move {
                    let _permit = sem_clone.acquire_owned().await.unwrap();
                    let mut ai_acts = Vec::with_capacity(batch_size);
                    // TODO macro this?
                    for e in batch {
                        if let Some(means) = e.write().await.tick(r_tick, r_env, r_ter) {
                            for m in means {
                                if let TickMeaning::AiStateChange { maybe_action: Some(act),.. } = m {
                                    #[cfg(feature = "stresstest")]{ unsafe { AISC += 1; } }
                                    ai_acts.push(act);
                                }
                            }
                        }
                    }
                    ai_acts
                });
            }
        }
        // …any stragglers?
        if !curr_ent_batch.is_empty() {
            let sem_clone = Arc::clone(&self.sem);
            let r_env = self.special_environment;
            let r_ter = self.terrain.clone();
            let r_tick = curr_tick;
            let batch_len = curr_ent_batch.len();
            join_set.spawn(async move {
                let _permit = sem_clone.acquire_owned().await.unwrap();
                let mut ai_acts = Vec::with_capacity(batch_len);
                // TODO see case #1 higher above about macro…
                for e in curr_ent_batch {
                    if let Some(means) = e.write().await.tick(r_tick, r_env, r_ter) {
                        for m in means {
                            if let TickMeaning::AiStateChange { maybe_action: Some(act),.. } = m {
                                #[cfg(feature = "stresstest")]{ unsafe { AISC += 1; } }
                                ai_acts.push(act);
                            }
                        }
                    }
                }
                ai_acts
            });
        }

        #[cfg(feature = "stresstest")] static mut MIREC: usize = 0;
        let mut emohash: HashMap<&str, (MachineId, usize)> = HashMap::new();
        while let Some(ai_act_res) = join_set.join_next().await {
            if let Ok(ai_acts) = ai_act_res {
                for act in ai_acts.into_iter() {
                    if let AiAction::Emote { ent_m_id, fmt } = act {
                        #[cfg(feature = "stresstest")] unsafe {
                            MIREC += 1;
                            if MIREC < 10 {
                                log::debug!("BCAST::MIRE");
                            } else if MIREC % 1_000_000 == 0 {
                                let mirec = MIREC;
                                log::debug!("BCAST::MIRE ×{mirec}")
                            }
                        }
                        let (_,c) = emohash.entry(fmt).or_insert((ent_m_id, 0));
                        *c += 1;
                    }
                }
            }
        }

        for (fmt, (ent_m_id, count)) in emohash {
            self.out.send(Broadcast::MessageInRoomE {
                room: room.clone(),
                entity: ent_m_id,
                message: match count {
                    1 => fmt.to_string(),
                    _ => fmt.replace("~e~", &format!("~e~ ×{count}"))
                }
            }).ok();
        }

        // no reaction yet to "positive" tick(s)
        let _ = self.contents.tick(curr_tick, self.special_environment, self.terrain);
        #[cfg(all(debug_assertions,feature = "stresstest"))]{
            log::debug!("Room '{}' ticked.", self.id);
        }
    }
}
