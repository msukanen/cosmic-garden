//! Player stuff!

use std::{collections::HashMap, fmt::Display, sync::{Arc, Weak}};

use cosmic_garden_pm::{IdentityMut, MobMut};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::{error::Error, identity::IdentityQuery, io::{ClientState, SAVE_PATH}, item::{Item, consumable::NutritionType, container::{Storage, StorageError, variants::{ContainerVariant, ContainerVariantType}}}, mob::{Stat, StatType, StatValue, affect::Affect, traits::MobMut}, room::Room, string::UNNAMED, thread::janitor::{SAVE_ASAP, SAVE_ASAP_THRESHOLD}, traits::Tickable, util::{HelpPage, access::{Access, Accessor}, activity::ActionWeight, config::Config}, world::World};

#[derive(Debug, Clone, PartialEq)]
pub enum ActivityType {
    Building,
    Playing,
    Other
}

impl Default for ActivityType {
    fn default() -> Self {
        Self::Other
    }
}

impl Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Building => "<c red>BLD</c>",
            Self::Playing => "<c green>PLY</c>",
            Self::Other => "<c yellow>OTH</c>",
        })
    }
}

/// A player's character contained here…
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, MobMut)]
pub struct Player {
    /// ID of owner of this specific [Player] character.
    pub(super) owner_id: String,
    
    id: String,
    #[identity(title)]
    pub(super) name: String,
    
    #[serde(default)]
    pub config: Config,
    #[serde(default)]
    pub access: Access,
    
    #[serde(default, skip)]
    pub actions_taken: usize,
    #[serde(default = "player_location_void")]
    pub(crate) location_id: String,
    #[serde(skip)]
    pub location: Weak<RwLock<Room>>,
    #[serde(default = "player_hp_default")]
    pub hp: Stat,
    #[serde(default = "player_mp_default")]
    pub mp: Stat,
    #[serde(default = "player_sn_default")]
    pub sn: Stat,
    #[serde(default = "player_san_default")]
    pub san: Stat,
    
    #[serde(default)] pub redit_buffer: Option<Room>,
    #[serde(default)] pub iedit_buffer: Option<Item>,
    #[serde(default)] pub hedit_buffer: Option<HelpPage>,
    
    #[serde(default = "player_default_atype", skip)]
    pub activity_type: ActivityType,

    #[serde(default = "player_inv_default")]
    pub inventory: ContainerVariant,

    #[serde(default)]
    pub affects: HashMap<String, Affect>,
}

fn player_location_void() -> String { UNNAMED.into() }
fn player_hp_default() -> Stat { Stat::new(StatType::HP) }
fn player_mp_default() -> Stat { Stat::new(StatType::MP) }
fn player_sn_default() -> Stat { Stat::new(StatType::SN) }
fn player_san_default() -> Stat { Stat::new(StatType::San) }
fn player_default_atype() -> ActivityType { ActivityType::default() }
fn player_inv_default() -> ContainerVariant {
    ContainerVariant::raw(ContainerVariantType::PlayerInventory)
}

impl Player {
    pub fn owner_id<'a>(&'a self) -> &'a str { &self.owner_id }

    pub async fn load(owner_id: &str, id: &str) -> Result<Arc<RwLock<Self>>, Error> {
        let mut player: Self = serde_json::from_str(
            &fs::read_to_string(&format!("{}/{}-{}.player", SAVE_PATH.display(), owner_id, id)).await?
        )?;
        player.activity_type = ActivityType::Playing;
        Ok(Arc::new(RwLock::new(player)))
    }

    pub async fn save(&self) -> Result<(), Error> {
        let path = format!("{}/{}-{}.player", SAVE_PATH.display(), self.owner_id, self.id);
        #[cfg(feature = "stresstest")]
        log::debug!("Storing {path}");
        fs::write(path, serde_json::to_string_pretty(self)?).await?;
        Ok(())
    }

    pub fn prompt(&self, state: &ClientState) -> Option<String> {
        match state {
            ClientState::Playing { .. } => format!("[{} {} {}]#> ", self.hp, self.mp, self.sn).into(),
            ClientState::Editing { mode, .. } => mode.prompt(&self).into(),
            _ => None// all other states are dealt by I/O machinery directly.
        }
    }

    pub async fn place(player: Arc<RwLock<Player>>, world: Arc<RwLock<World>>, target_id: &str) -> Result<(), Error> {
        if let Some(target_arc) = world.read().await.rooms.get(target_id) {
            Player::place_direct(player.clone(), target_arc.clone()).await?
        }
        Ok(())
    }

    pub async fn place_direct(player: Arc<RwLock<Player>>, target_arc: Arc<RwLock<Room>>) -> Result<(), Error> {
        let tgt_id = {
            let mut tgt_lock = target_arc.write().await;
            let p_lock = player.read().await;
            tgt_lock.who.insert(p_lock.id().into(), Arc::downgrade(&player));
            tgt_lock.id().to_string()
        };
        if let Some(origin) = player.read().await.location.upgrade() {
            let mut origin_lock = origin.write().await;
            origin_lock.who.remove(player.read().await.id());
        }
        // some arcrobatics was needed around places to make this part not to deadlock...
        {
            let mut p_lock = player.write().await;
            p_lock.location_id = tgt_id.clone();
            p_lock.location = Arc::downgrade(&target_arc);
        }
        log::trace!("Placed player at '{}'", tgt_id);
        Ok(())
    }

    /// Accumulate action weight.
    pub async fn act(&mut self, player: Arc<RwLock<Player>>, act_wt: ActionWeight) -> usize {
        self.actions_taken += act_wt;
        if self.actions_taken >= SAVE_ASAP_THRESHOLD {
            let mut asap = (*SAVE_ASAP).write().await;
            if !asap.iter().any(|existing| Arc::ptr_eq(existing, &player)) {
                asap.push(player.clone());
            }
        }
        self.actions_taken
    }

    /// Receive an item. If you can't take it, throw it back…
    pub fn receive_item(&mut self, item: Item) -> Result<(), StorageError> {
        self.inventory.try_insert(item)
    }

    /// Apply ± on a stat.
    pub fn apply_stat_change(&mut self, stat: StatType, drain: StatValue) {
        match stat {
            StatType::HP => *(self.hp_mut()) += drain,
            StatType::MP => *(self.mp_mut()) += drain,
            StatType::SN => *(self.sn_mut()) += drain,
            StatType::San => *(self.san_mut()) += drain,
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            owner_id: UNNAMED.into(),
            id: "".into(),
            name: "".into(),
            actions_taken: 0,
            location_id: player_location_void(),
            location: Weak::new(),
            access: Access::default(),
            hp: player_hp_default(),
            mp: player_mp_default(),
            sn: player_sn_default(),
            san: player_san_default(),
            config: Config::default(),
            redit_buffer: None,
            iedit_buffer: None,
            hedit_buffer: None,
            activity_type: ActivityType::Other,
            inventory: player_inv_default(),
            affects: HashMap::new(),
        }
    }
}

impl Accessor for Player {
    fn is_admin(&self) -> bool {
        self.access.is_admin()
    }

    fn is_builder(&self) -> bool {
        self.access.is_builder()
    }

    fn is_event_host(&self) -> bool {
        self.access.is_event_host()
    }

    fn is_true_builder(&self) -> bool {
        self.access.is_true_builder()
    }
}

impl Tickable for Player {
    fn tick(&mut self) -> bool {
        let hp_t = self.hp_mut().tick();
        let mp_t = self.mp_mut().tick();
        let sn_t = self.sn_mut().tick();
        let san_t = self.san_mut().tick();
        // now Affects, if any …
        let mut changes: Vec<_> = Vec::new();
        self.affects.retain(|_id, affect| {
            if affect.expired() { return false; }
            if let Affect::Nutrition { kind, .. } = affect {
                if let NutritionType::Heal { stat, drain } = kind {
                    changes.push((*stat, *drain));
                }
            }

            affect.tick();
            !affect.expired()
        });
        for (stat, amount) in &changes {
            self.apply_stat_change(*stat, *amount);
        }
        let ch_t = !changes.is_empty();
        let inv_t = self.inventory.tick();

        let meaningful = hp_t || mp_t || sn_t || san_t || ch_t || inv_t;
        #[cfg(debug_assertions)]{
            if meaningful {
                log::debug!("Player-ID '{}' ticked.", self.id);
            }
        }
        meaningful
    }
}
