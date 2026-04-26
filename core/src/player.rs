//! Player stuff!

use std::{collections::HashMap, fmt::Display, sync::{Arc, Weak}};

use async_trait::async_trait;
use cosmic_garden_pm::{CombatantMut, Factioned, IdentityMut, Mob, MobMut};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::{combat::{Combatant, CombatantMut, Damager}, error::CgError, identity::IdentityQuery, io::{ClientState, player_save_fp}, item::{Item, consumable::EffectType, container::{Storage, StorageError, variants::{ContainerVariant, ContainerVariantType}}, weapon::str_based_dmg_mul}, mob::{Stat, StatType, StatValue, affect::Affect, faction::{EntityFaction, FactionMut}, traits::Mob}, room::Room, string::UNNAMED, thread::{SystemSignal, janitor::SAVE_ASAP_THRESHOLD, signal::SignalSenderChannels}, traits::Tickable, util::{HelpPage, access::{Access, Accessor}, activity::ActionWeight, config::Config, direction::Direction}};

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
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, Mob, MobMut, CombatantMut, Factioned)]
pub struct Player {
    /// ID of owner of this specific [Player] character.
    pub(super) owner_id: String,
    
    id: String,
    #[identity(title)]
    pub(super) name: String,
    
    #[serde(default)] pub config: Config,
    #[serde(default)] pub access: Access,
    
    #[serde(default, skip)]
    pub actions_taken: usize,
    #[serde(default = "player_location_void")]
    pub(crate) location_id: String,
    #[serde(skip)]
    pub location: Weak<RwLock<Room>>,
    
    #[serde(default = "player_hp_default")] pub hp: Stat,
    #[serde(default = "player_mp_default")] pub mp: Stat,
    #[serde(default = "player_sn_default")] pub sn: Stat,
    #[serde(default = "player_san_default")] pub san: Stat,
    #[serde(default = "player_brn_default")] pub brn: Stat,
    #[serde(default = "player_nim_default")] pub nim: Stat,
    #[serde(default = "player_str_default")] pub strn: Stat,
    
    #[serde(default)] pub redit_buffer: Option<Room>,
    #[serde(default)] pub iedit_buffer: Option<Item>,
    #[serde(default)] pub hedit_buffer: Option<HelpPage>,
    
    #[serde(default = "player_default_atype", skip)]
    pub activity_type: ActivityType,

    #[serde(default = "player_inv_default")]
    pub inventory: ContainerVariant,

    /// Current affects.
    #[serde(default)]
    pub affects: HashMap<String, Affect>,

    /// Last place in the line of travels…
    #[serde(default, skip)]
    pub last_goto: Option<(Direction, Weak<RwLock<Room>>)>,

    #[serde(skip, default = "player_faction_default")]
    pub faction: EntityFaction,

    #[serde(default)]
    pub equipped_weapon: Option<Item>,

    #[serde(default = "player_rep_default")]
    pub reputation: Stat,

    /// Is the character 'hardcore', eligible for perma-death?
    #[serde(default)]
    pub hardcore: Option<bool>,
}

fn player_location_void() -> String { UNNAMED.into() }
fn player_hp_default() -> Stat { Stat::new(StatType::HP) }
fn player_mp_default() -> Stat { Stat::new(StatType::MP) }
fn player_sn_default() -> Stat { Stat::new(StatType::SN) }
fn player_san_default() -> Stat { Stat::new(StatType::San) }
fn player_brn_default() -> Stat { Stat::new(StatType::Brn) }
fn player_nim_default() -> Stat { Stat::new(StatType::Nim) }
fn player_str_default() -> Stat { Stat::new(StatType::Str) }
fn player_default_atype() -> ActivityType { ActivityType::default() }
pub(crate) fn player_inv_default() -> ContainerVariant {
    ContainerVariant::raw(ContainerVariantType::PlayerInventory)
}
fn player_faction_default() -> EntityFaction { EntityFaction::Player { pvp: false }}
fn player_rep_default() -> Stat { Stat::Rep { curr: 0.0 }}

impl Player {
    pub fn owner_id<'a>(&'a self) -> &'a str { &self.owner_id }

    /// Attempt to load a [Player].
    /// 
    /// # Args
    /// - `owner_id` of the [user][crate::user::UserInfo].
    /// - `id` of the [Player].
    /// 
    /// # Returns
    /// `Arc<RwLock<Player>>` if successful.
    pub async fn load(owner_id: &str, id: &str) -> Result<Arc<RwLock<Self>>, CgError> {
        let mut player: Self = serde_json::from_str(
            &fs::read_to_string(player_save_fp(owner_id, id)).await?
        )?;
        player.activity_type = ActivityType::Playing;
        Ok(Arc::new(RwLock::new(player)))
    }

    /// Attempt to save self…
    pub async fn save(&self) -> Result<(), CgError> {
        fs::write(player_save_fp(self.owner_id(), self.id()),
        serde_json::to_string_pretty(self)?).await?;
        Ok(())
    }

    /// Show current playing state's prompt, if possible.
    pub fn prompt(&self, state: &ClientState) -> Option<String> {
        match state {
            ClientState::Playing { .. } => format!("[{} {} {}]#> ", self.hp, self.mp, self.sn).into(),
            ClientState::Editing { mode, .. } => mode.prompt(&self).into(),
            _ => None// all other states are dealt by I/O machinery directly.
        }
    }

    /// Place [Player] directly in `target_arc` [Room].
    /// 
    //NOTE: potential deadlock if not careful.
    pub async fn place_direct(player: Arc<RwLock<Player>>, target_arc: Arc<RwLock<Room>>) -> Result<(), CgError> {
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
        // some arcrobatics was needed around places to make this part to not deadlock…
        {
            let mut p_lock = player.write().await;
            p_lock.location_id = tgt_id.clone();
            p_lock.location = Arc::downgrade(&target_arc);
        }
        log::trace!("Placed player at '{}'", tgt_id);
        Ok(())
    }

    /// Accumulate action weight.
    pub async fn act(&mut self, player: Arc<RwLock<Player>>, system_ch: &SignalSenderChannels, act_wt: ActionWeight) -> usize {
        self.actions_taken += act_wt.clone();
        if self.actions_taken >= SAVE_ASAP_THRESHOLD {
            // He'll pick up, sooner or later…
            system_ch.janitor.send(SystemSignal::PlayerNeedsSaving(player.clone())).ok();
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
            StatType::Brn => *(self.brn_mut()) += drain,
            StatType::HP  => *(self.hp_mut())  += drain,
            StatType::MP  => *(self.mp_mut())  += drain,
            StatType::Nim => *(self.nim_mut()) += drain,
            StatType::SN  => *(self.sn_mut())  += drain,
            StatType::San => *(self.san_mut()) += drain,
            StatType::Str => *(self.str_mut()) += drain,
            StatType::Rep => *(self.rep_mut()) += drain,
        }
    }

    /// Purge all the editor buffers without care.
    pub fn purge_buffers(&mut self) {
        self.hedit_buffer = None;
        self.iedit_buffer = None;
        self.redit_buffer = None;
    }

    /// Get mutable [reputation][Stat].
    pub fn rep_mut(&mut self) -> &mut Stat {
        &mut self.reputation
    }

    /// Step toward 'hardcore' mode, or switch it on.
    /// 
    /// This is an irreversible operation (sans admin intervention)…
    /// 
    /// # Returns
    /// - `false`: pending
    /// - `true`: set
    pub fn step_hardcore(&mut self) -> bool {
        let mut set = false;
        self.hardcore = match self.hardcore {
            None => Some(false),
            _ => { set = true; Some(true) }
        };
        set
    }
}

impl Default for Player {
    /// Construt a default [Player] instance.
    fn default() -> Self {
        Self {
            owner_id: UNNAMED.into(),
            id: "***".into(),
            name: "***".into(),
            actions_taken: 0,
            location_id: player_location_void(),
            location: Weak::new(),
            access: Access::default(),
            hp: player_hp_default(),
            mp: player_mp_default(),
            sn: player_sn_default(),
            san: player_san_default(),
            brn: player_brn_default(),
            nim: player_nim_default(),
            strn: player_str_default(),
            reputation: Stat::Rep { curr: 0.0 },
            config: Config::default(),
            redit_buffer: None,
            iedit_buffer: None,
            hedit_buffer: None,
            activity_type: ActivityType::Other,
            inventory: player_inv_default(),
            affects: HashMap::new(),
            last_goto: None,
            faction: EntityFaction::Player { pvp: false },
            equipped_weapon: None,
            hardcore: None,
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

#[async_trait]
impl Tickable for Player {
    async fn tick(&mut self) -> bool {
        let hp_t = self.hp_mut().tick().await;
        let mp_t = self.mp_mut().tick().await;
        let sn_t = self.sn_mut().tick().await;
        let san_t = self.san_mut().tick().await;
        let old_affects = std::mem::take(&mut self.affects);
        let mut survivors = HashMap::new();
        let mut changes: Vec<_> = Vec::new();
        for (id, mut affect) in old_affects {
            if affect.expired() { continue; }
            {
                if let Affect::Effect { ref kind, .. } = affect {
                    if let EffectType::Heal { stat, drain } = kind {
                        changes.push((*stat, *drain));
                    }
                }
            }
            let hc = matches!(affect, Affect::HardcorePending { .. });
            affect.tick().await;
            if affect.expired() && hc {
                if let Some(false) = self.hardcore {
                    self.hardcore = None;
                }
            }
            if !affect.expired() {
                survivors.insert(id, affect);
            }
        }
        self.affects = survivors;
        for (stat, amount) in &changes {
            self.apply_stat_change(*stat, *amount);
        }
        let ch_t = !changes.is_empty();
        let inv_t = self.inventory.tick().await;

        let meaningful = hp_t || mp_t || sn_t || san_t || ch_t || inv_t;
        #[cfg(debug_assertions)]{
            if meaningful {
                log::debug!("Player-ID '{}' ticked.", self.id);
            }
        }
        meaningful
    }
}

impl FactionMut for Player {
    fn faction_mut(&mut self) -> &mut EntityFaction {
        &mut self.faction
    }
}

impl Damager for Player {
    fn dmg(&self) -> StatValue {
        let Some(Item::Weapon(w)) = &self.equipped_weapon else {
            return self.str() / 100.0;// Str(S)/100; S=100 by default (for human at least).
        };
        
        // W × Str(S)/50; S=100 by default (for human at least).
        w.base_dmg * str_based_dmg_mul(self.str().current(), false) * (self.size().rel_vs_weapon(&w.weapon_size))
    }
}
