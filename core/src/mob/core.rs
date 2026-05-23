//! Mob core.

use std::{fmt::Display, sync::RwLock};

use async_trait::async_trait;
use cosmic_garden_pm::{CombatantMut, DescribableMut, FactionMut, IdentityMut, Mob, MobMut};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{
    combat::{Battler, Combatant, CombatantMut, DamageType, Damager}, r#const::STAT_PULSE_NTH_TICK, error::CgError, identity::{IdentityQuery, MachineId, MachineIdentity, uniq::{StrUuid, UuidCore}}, io::entity_entry_fp, item::{
        Item, StorageSpace, container::variants::{ContainerVariant, ContainerVariantType}, weapon::{WeaponSize, str_based_dmg_mul}
    }, mob::{Ai, EntityArc, Gender, GenderError, GenderType, Stat, StatType, StatValue, ai::{AiAction, AiMentalState}, faction::{Demeanor, EntityFaction}, traits::MobMut}, room::{RoomWeak, environ::{SpecialEnvironment, Terrain}}, string::UNNAMED, thread::{librarian::get_entity_blueprint, signal::SignalSenderChannels}, traits::{TickMeaning, Tickable}
};

/// Generic [Entity] size categories
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum EntitySize {
    VeryTiny,
    Tiny,
    Small,
    Medium,// "human"
    Large,
    Huge,
    Gargantuan,
}

impl EntitySize {
    /// Relative size vs a [weapon][WeaponSize].
    pub fn rel_vs_weapon(&self, weapon_size: &WeaponSize) -> f32 {
        let a_idx = i8::from(self);
        let w_idx = i8::from(weapon_size);
        match (a_idx - w_idx).abs() {
            0 => 1.0, // perfect match
            1 => 0.9, // slightly off, human with 2h or a dagger
            2 => 0.6, // awkward, human with a needle or huge polearm
            3 => 0.3, // ridonkylous, tiny pixie with a huge polearm
            4 => 0.1, // …near impossible
            _ => 0.05 // …quite impossible
        }
    }
}

impl From<&EntitySize> for StorageSpace {
    fn from(value: &EntitySize) -> Self {
        match value {
            EntitySize::Gargantuan => 1_000,
            EntitySize::Huge => 500,
            EntitySize::Large => 200,
            EntitySize::Medium => 100,
            EntitySize::Small => 50,
            EntitySize::Tiny => 25,
            EntitySize::VeryTiny => 10,
        }
    }
}

impl From<&EntitySize> for i8 {
    fn from(value: &EntitySize) -> Self {
        match value {
            EntitySize::VeryTiny => -3,
            EntitySize::Tiny => -2,
            EntitySize::Small => -1,
            EntitySize::Medium => 0,
            EntitySize::Large => 1,
            EntitySize::Huge => 2,
            EntitySize::Gargantuan => 4,
        }
    }
}

impl Display for EntitySize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Gargantuan => "gargantuan",
            Self::Huge => "huge",
            Self::Large => "large",
            Self::Medium => "medium",
            Self::Small => "small",
            Self::Tiny => "tiny",
            Self::VeryTiny => "very tiny",
        })
    }
}

#[derive(Debug)]
pub enum EntitySizeError {
    NotSize(String),
    VoidSize,
}

impl Display for EntitySizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VoidSize => write!(f, "An empty value does not represent any kind of size/stature…"),
            Self::NotSize(v) => write!(f, "'{v}' is not any recognizeable size/stature…"),
        }
    }
}

impl TryFrom<&str> for EntitySize {
    type Error = EntitySizeError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.chars().nth(0) {
            Some(v) => Ok(match v {
                'g'|'G' => Self::Gargantuan,
                'h'|'H' => Self::Huge,
                'l'|'L' => Self::Large,
                'm'|'M' => Self::Medium,
                's'|'S' => Self::Small,
                't'|'T' => Self::Tiny,
                'v'|'V' => Self::VeryTiny,
                _ => return Err(EntitySizeError::NotSize(value.into()))
            }),

            _ => Err(EntitySizeError::VoidSize)
        }
    }
}

/// An entity of some sort…
#[derive(Debug, Clone, DescribableMut, Deserialize, Serialize, IdentityMut, Mob, MobMut, FactionMut, CombatantMut)]
pub struct Entity {
    id: String,
    /// Tick-ID, used for the entity's [AI][Ai], etc.
    // `tick_id` is derived from the [Entity]'s `Arc<RwLock<..>>`.
    #[serde(skip)] tick_id: MachineId,
    #[identity(title)] name: String,
    #[serde(default, skip)] location: RoomWeak,
    desc: String,
    #[serde(default)] gender: GenderType,
    hp: Stat,
    mp: Stat,
    san: Stat,
    sn: Stat,
    brn: Stat,
    nim: Stat,
    strn: Stat,
    satiation: Stat,
    faction: EntityFaction,
    max_weapon_size: WeaponSize,
    size: EntitySize,
    pub(crate) equipped_weapon: Option<Item>,
    #[serde(default = "entity_inv_default")] inventory: ContainerVariant,
    
    // AI stuff…
    #[serde(default, skip)] brain_freeze: bool,
    #[serde(default)] ai: Ai,
    // tick scatter…
    #[serde(skip, default)] last_stat_tick: usize,
    #[serde(skip, default)] last_ai_tick: usize,
    #[serde(skip, default)] last_inv_tick: usize,
}

impl Default for Entity {
    fn default() -> Self {
        let id = "entity".with_uuid();
        Self {
            tick_id: id.as_m_id(),
            id,
            name: UNNAMED.into(),
            hp: Stat::new(StatType::HP),
            mp: Stat::new(StatType::MP),
            san: Stat::new(StatType::San),
            sn: Stat::new(StatType::SN),
            brn: Stat::new(StatType::Brn),
            nim: Stat::new(StatType::Nim),
            strn: Stat::new(StatType::Str),
            satiation: Stat::new(StatType::Sat),
            faction: EntityFaction::NPC { demeanor: Demeanor::default() },
            max_weapon_size: WeaponSize::Large,
            equipped_weapon: None,
            size: EntitySize::Medium,
            location: std::sync::Weak::new(),
            inventory: entity_inv_default(),
            brain_freeze: false,
            desc: "Some sort of an entity. Use <c yellow>desc =</c> to describe it…".into(),
            gender: GenderType::default(),
            ai: Ai::default(),
            last_stat_tick: 0,
            last_ai_tick: 0,
            last_inv_tick: 0,
        }
    }
}

fn entity_inv_default() -> ContainerVariant {
    ContainerVariant::raw(ContainerVariantType::Corpse)
}

#[derive(Debug)]
pub enum EntityError {
    NoSuchEntity(String)
}

impl Display for EntityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchEntity(e) => write!(f, "No such entity as '{e}'")
        }
    }
}

impl std::error::Error for EntityError {}

impl Entity {
    pub async fn new(id: &str, out: &SignalSenderChannels) -> Result<Self, CgError> {
        if let Some(ent) = get_entity_blueprint(id, out).await {
            let id = id.show_uuid(false).to_string();
            return Ok(Self {
                tick_id: id.as_m_id(),
                id,
                ..ent
            })
        }
        
        Err(CgError::from(EntityError::NoSuchEntity(id.into())))
    }

    /// Save the entity blueprint.
    pub async fn save_bp(&self) -> Result<(), CgError> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(entity_entry_fp(self.id().show_uuid(false)), contents).await?;
        Ok(())
    }

    /// Create a shallow clone of self (for builders, mainly).
    pub fn shallow_clone(&self) -> Self {
        Self {
            inventory: entity_inv_default(),
            desc: self.desc.clone(),
            location: std::sync::Weak::new(),
            brain_freeze: false,
            equipped_weapon: None,
            ..self.clone()
        }
    }

    /// Copyback from another `ent`. Generally from builder's editor buffer…
    pub fn copyback(&mut self, ent: Entity) {
        self.name = ent.name;
        self.hp = ent.hp;
        self.mp = ent.mp;
        self.san = ent.san;
        self.sn = ent.sn;
        self.brn = ent.brn;
        self.nim = ent.nim;
        self.strn = ent.strn;
        self.faction = ent.faction;
        self.max_weapon_size = ent.max_weapon_size;
        self.size = ent.size;
        self.desc = ent.desc;
    }

    pub fn stat_mut(&mut self, stat_type: &StatType) -> &mut Stat {
        match stat_type {
            StatType::Brn => &mut self.brn,
            StatType::HP => &mut self.hp,
            StatType::MP => &mut self.mp,
            StatType::Nim => &mut self.nim,
            StatType::SN => &mut self.sn,
            StatType::Str => &mut self.strn,
            StatType::San => &mut self.san,
            StatType::Sat => &mut self.satiation,
            StatType::Rep => unimplemented!("Entity do not have 'reputation' stat."),
        }
    }

    /// Set the [Entity]'s `tick_id`.
    pub fn set_tick_id(&mut self, self_arc: &EntityArc) -> MachineId {
        self.tick_id = lock2key!(arc self_arc);
        self.tick_id
    }

    /// Check whether the [Entity] wants to attack one of the [Battler]s.
    pub async fn maybe_attack_one<'a>(&self, vcts: &'a Vec<Battler>) -> Option<&'a Battler> {
        // TODO figure out if any of the victim candidates suit as target practice…
        let (hp_threshold_self, hp_threshold_other) = match self.ai.mental_state {
            AiMentalState::Angry => (0.15, 1.0),
            AiMentalState::Grumpy => (0.5, 0.75),
            _ => (0.9, 0.2)
        };
        if self.hp.current() / self.hp.max() < hp_threshold_self {
            // too hurt to want to initiate (new) fight(s)
            return None;
        }
        for v in vcts {
            let vl = v.read().await;
            if vl.hp().current() / vl.hp().max() <= hp_threshold_other {
                log::debug!("Found hurties");
                return v.into();
            }
        }
        None
    }
}

impl Damager for Entity {
    fn dmg(&self) -> StatValue {
        let Some(Item::Weapon(w)) = &self.equipped_weapon else { return 1.0 * self.str() / 100.0 };
        w.base_dmg * str_based_dmg_mul(self.str().current(), true) * (self.size.rel_vs_weapon(&w.weapon_size))
    }

    fn dmg_type(&self) -> crate::combat::DamageType {
        let Some(Item::Weapon(w)) = &self.equipped_weapon else { return DamageType::Crush; };
        w.dmg_type()
    }
}

#[async_trait]
impl Tickable for Entity {
    fn tick(&mut self, curr_tick: usize, room_env: SpecialEnvironment, room_terrain: Option<Terrain>) -> Option<Vec<TickMeaning>> {
        // tick stats at 1/10th of our [Room]'s pace.
        if should_pulse!(curr_tick, self.last_stat_tick, self.tick_id, STAT_PULSE_NTH_TICK) {
            self.last_stat_tick = curr_tick;
            // we tick just the drainable stats.
            self.hp_mut().tick(curr_tick, room_env, room_terrain);
            self.mp_mut().tick(curr_tick, room_env, room_terrain);
            self.sn_mut().tick(curr_tick, room_env, room_terrain);
            self.san_mut().tick(curr_tick, room_env, room_terrain);
            self.satiation_mut().tick(curr_tick, room_env, room_terrain);
        }

        #[cfg(feature = "stresstest")] static mut AIMC: usize = 0;
        // tick AI at 1/15th of our [Room]'s pace.
        let maybe_ai_meaning =
        if !self.brain_freeze && should_pulse!(curr_tick, self.last_ai_tick, self.tick_id, 15) {
            self.last_ai_tick = curr_tick;
            if let Some(ai_means) = self.ai.tick(
                self.tick_id(),
                curr_tick,
                room_env,
                room_terrain,
                self.faction,
            ) {
                if let TickMeaning::AiStateChange { maybe_action: Some(_), .. } = &ai_means {
                    #[cfg(feature = "stresstest")]{
                        unsafe {
                            if AIMC <= 10 {
                                AIMC += 1;
                                log::debug!("Entity@AiStateChange::Emote");
                            }
                        }
                    }
                    ai_means.into()
                } else { None }
            } else { None }
        } else { None }
        ;

        // tick inventory at 1/25th the [Room]'s pace.
        let inv_m = if should_pulse!(curr_tick, self.last_inv_tick, self.tick_id, 25) {
            self.last_inv_tick = curr_tick;
            self.inventory.tick(curr_tick, room_env, room_terrain)
        } else { None };

        match (maybe_ai_meaning, inv_m) {
            (None, None) => None,
            (Some(a), None) => vec![a].into(),
            (None, Some(b)) => b.into(),
            (Some(a), Some(mut b)) => { b.push(a); b.into() }
        }
    }
}

impl Gender for Entity {
    fn gender(&self) -> GenderType { self.gender }
    fn set_gender(&mut self, gender: GenderType) -> Result<(), GenderError> {
        self.gender = gender;
        Ok(())
    }
}

#[cfg(test)]
mod entity_tests {
    use std::io::Cursor;

    use crate::{cmd::look::LookCommand, combat::{Combatant, CombatantMut}, get_operational_mock_librarian, get_operational_mock_life, identity::{IdentityMut, IdentityQuery, uniq::{UUID_RE, Uuid}}, mob::core::Entity, stabilize_threads, string::UNNAMED, thread::{SystemSignal, signal::SpawnType}, traits::Tickable, util::access::Access, world::mock_world::get_operational_mock_world};

    #[cfg(feature = "stresstest")]
    const LOOPS: u32 = 1_000_000;
    #[cfg(not(feature = "stresstest"))]
    const LOOPS: u32 = 1_000;

    #[tokio::test]
    async fn entity_default() {
        let _ = env_logger::try_init();
        let now = std::time::Instant::now();
        let mut e = Entity::default();
        assert!(UUID_RE.is_match(e.id()));
        assert!(e.id().starts_with("entity-"));
        assert_eq!(UNNAMED, e.title());
        e.mp_mut().set_drain(-1.0);
        log::debug!("re-UUID is heavy (Uuid::new_v4()), and it'd never be used in a loop like this in reality, but… hold the press until {LOOPS} x 100 ticks is done.:");
        let mut tick = 0;
        for x in 0..LOOPS {
            if x % 1_000 == 0 {
                log::debug!("Tickage… {x}");
            }
            let old_id = e.id().to_string();
            _ = e.set_id(&old_id.re_uuid(), true);
            assert_ne!(old_id.as_str(), e.id());
            e.mp_mut().set_curr(100.0);
            while !e.is_unconscious() {
                e.tick(tick, 0, None);
                tick += 1;
            }
            assert!(e.is_unconscious(), "Entity should be unconscious!");
        }
        let elapsed = now.elapsed();
        log::debug!("\nPERF: {LOOPS} reuuid + drain, 100 ticks each loop: {elapsed:?}\nPERF: avg per cycle: {:?}\nTOT: {} iterations.", elapsed / LOOPS, LOOPS*100);
    }

    #[tokio::test]
    async fn entity_save() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        stabilize_threads!(25);
        get_operational_mock_librarian!(c,w);
        get_operational_mock_life!(c,w);
        stabilize_threads!();
        let Ok(mob) = Entity::new("goblin", &c.out).await else {
            panic!("Where'd the lil goblin go?!");
        };
        if let Err(e) = mob.save_bp().await {
            panic!("goblin fail: {e:?}");
        }
        let _ = c.out.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None });
        stabilize_threads!(100);
        let state = ctx!(state, LookCommand, "",s,c.out,w,|out:&str| out.contains("goblin is here"));
        p.write().await.config.show_id = true;
        p.write().await.access = Access::Builder;
        let _ = ctx!(state, LookCommand, "",s,c.out,w,|out:&str| out.contains("(") && out.contains(")"));
    }
}
