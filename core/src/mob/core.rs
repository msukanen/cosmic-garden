//! Mob core.

use std::{fmt::Display, sync::Weak};

use async_trait::async_trait;
use cosmic_garden_pm::{CombatantMut, DescribableMut, FactionMut, IdentityMut, Mob, MobMut};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::{combat::{Combatant, CombatantMut, Damager}, error::CgError, identity::IdentityQuery, io::entity_entry_fp, item::{Item, container::variants::{ContainerVariant, ContainerVariantType}, weapon::{WeaponSize, str_based_dmg_mul}}, mob::{Stat, StatType, StatValue, faction::EntityFaction}, room::Room, string::{StrUuid, UNNAMED, as_id_with_uuid}, thread::{librarian::get_entity_blueprint, signal::SignalSenderChannels}, traits::Tickable};

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

#[derive(Debug, Clone, DescribableMut, Deserialize, Serialize, IdentityMut, Mob, MobMut, FactionMut, CombatantMut)]
pub struct Entity {
    id: String,
    #[identity(title)]
    name: String,
    desc: String,
    hp: Stat,
    mp: Stat,
    san: Stat,
    sn: Stat,
    brn: Stat,
    nim: Stat,
    strn: Stat,
    faction: EntityFaction,
    max_weapon_size: WeaponSize,
    size: EntitySize,
    pub(crate) equipped_weapon: Option<Item>,
    #[serde(default, skip)]
    location: Weak<RwLock<Room>>,
    #[serde(default = "entity_inv_default")]
    inventory: ContainerVariant,
    #[serde(default, skip)]
    brain_freeze: bool,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            id: as_id_with_uuid("entity").unwrap(),
            name: UNNAMED.into(),
            hp: Stat::new(StatType::HP),
            mp: Stat::new(StatType::MP),
            san: Stat::new(StatType::San),
            sn: Stat::new(StatType::SN),
            brn: Stat::new(StatType::Brn),
            nim: Stat::new(StatType::Nim),
            strn: Stat::new(StatType::Str),
            faction: EntityFaction::Neutral,
            max_weapon_size: WeaponSize::Large,
            equipped_weapon: None,
            size: EntitySize::Medium,
            location: Weak::new(),
            inventory: entity_inv_default(),
            brain_freeze: false,
            desc: "Some sort of an entity. Use <c yellow>desc =</c> to describe it…".into()
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
    #[cfg(test)]
    pub fn re_uuid(&mut self) {
        use crate::{identity::{IdentityMut, IdentityQuery}, string::uuid::Uuid};
        *self.id_mut() = self.id().re_uuid()
    }

    pub async fn new(id: &str, out: &SignalSenderChannels) -> Result<Self, CgError> {
        if let Some(ent) = get_entity_blueprint(id, out).await {
            return Ok(Self {
                id: id.show_uuid(false).into(),
                ..ent
            });
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
            location: Weak::new(),
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
            StatType::Rep => unimplemented!("Entity do not have 'reputation' stat."),
        }
    }
}

impl Damager for Entity {
    fn dmg(&self) -> StatValue {
        let Some(Item::Weapon(w)) = &self.equipped_weapon else { return 1.0 * self.str() / 100.0 };
        w.base_dmg * str_based_dmg_mul(self.str().current(), true) * (self.size.rel_vs_weapon(&w.weapon_size))
    }
}

#[async_trait]
impl Tickable for Entity {
    async fn tick(&mut self) -> bool {
        let hp = self.hp_mut().tick().await;
        let mp = self.mp_mut().tick().await;
        let sn = self.sn_mut().tick().await;
        let san = self.san_mut().tick().await;
        hp || mp || sn || san
    }
}

#[cfg(test)]
mod entity_tests {
    use std::io::Cursor;

    use crate::{stabilize_threads, cmd::look::LookCommand, combat::{Combatant, CombatantMut}, get_operational_mock_librarian, get_operational_mock_life, identity::IdentityQuery, mob::core::Entity, string::{UNNAMED, UUID_RE}, thread::{SystemSignal, signal::SpawnType}, traits::Tickable, util::access::Access, world::world_tests::get_operational_mock_world};

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
        for _ in 0..LOOPS {
            let old_id = e.id().to_string();
            e.re_uuid();
            assert_ne!(old_id.as_str(), e.id());
            let mut next_val = 100.0;
            e.mp_mut().set_curr(next_val);
            while next_val > 0.0 {
                next_val -= 1.0;
                if !e.tick().await {
                    panic!("No tick?!");
                }
                assert_eq!(next_val, e.mp());
            }
            assert_eq!(Ok(true), e.is_unconscious());
        }
        let elapsed = now.elapsed();
        log::debug!("\nPERF: {LOOPS} reuuid + drain, 100 ticks each loop: {elapsed:?}\nPERF: avg per cycle: {:?}\nTOT: {} iterations.", elapsed / LOOPS, LOOPS*100);
    }

    #[tokio::test]
    async fn entity_save() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        get_operational_mock_librarian!(c,w);
        get_operational_mock_life!(c,w);
        stabilize_threads!();
        let Ok(mob) = Entity::new("goblin", &c.out).await else {
            panic!("Where'd the lil goblin go?!");
        };
        if let Err(e) = mob.save_bp().await {
            panic!("goblin fail: {e:?}");
        }
        let _ = c.out.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into() });
        stabilize_threads!(100);
        let state = ctx!(state, LookCommand, "",s,c.out,w,p,|out:&str| out.contains("goblin is here"));
        p.write().await.config.show_id = true;
        p.write().await.access = Access::Builder;
        let _ = ctx!(state, LookCommand, "",s,c.out,w,p,|out:&str| out.contains("goblin-"));
    }
}
