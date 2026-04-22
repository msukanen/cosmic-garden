//! Mob core.

use cosmic_garden_pm::{CombatantMut, FactionMut, IdentityMut, Mob, MobMut};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{combat::{Combatant, Damager}, error::CgError, identity::{IdentityMut, IdentityQuery}, io::entity_entry_fp, item::{Item, weapon::{WeaponSize, str_based_dmg_mul}}, mob::{Stat, StatType, StatValue, faction::EntityFaction}, string::{StrUuid, UNNAMED, as_id_with_uuid}, thread::librarian::ENT_BP_LIBRARY};

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

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, Mob, MobMut, FactionMut, CombatantMut)]
pub struct Entity {
    id: String,
    #[identity(title)]
    name: String,
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
    equipped_weapon: Option<Item>,
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
        }
    }
}

impl Entity {
    #[cfg(test)]
    pub fn re_uuid(&mut self) {
        use crate::{identity::{IdentityMut, IdentityQuery}, string::uuid::Uuid};
        *self.id_mut() = self.id().re_uuid()
    }

    pub async fn new(id: &str) -> Result<Self, CgError> {
        let Some(mut ent) = (*ENT_BP_LIBRARY).read().await.get(&id) else {
            return Ok(Self {
                id: id.show_uuid(false).into(),
                ..Self::default()
            });
        };

        *(ent.id_mut()) = id.show_uuid(false).into();
        Ok(ent)
    }

    /// Save the entity blueprint.
    pub async fn save_bp(&self) -> Result<(), CgError> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(entity_entry_fp(self.id().show_uuid(false)), contents).await?;
        Ok(())
    }
}

impl Damager for Entity {
    fn dmg(&self) -> StatValue {
        let Some(Item::Weapon(w)) = &self.equipped_weapon else { return 1.0 * self.str() / 100.0 };
        w.base_dmg * str_based_dmg_mul(self.str().current(), true) * (self.size.rel_vs_weapon(&w.weapon_size))
    }
}

#[cfg(test)]
mod entity_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::look::LookCommand, combat::{Combatant, CombatantMut}, get_operational_mock_librarian, get_operational_mock_life, identity::IdentityQuery, io::ClientState, mob::core::Entity, string::{UNNAMED, UUID_RE}, thread::{SystemSignal, librarian::librarian, life::life, signal::SpawnType}, traits::Tickable, util::access::Access, world::world_tests::get_operational_mock_world};

    #[cfg(feature = "stresstest")]
    const LOOPS: u32 = 1_000_000;
    #[cfg(not(feature = "stresstest"))]
    const LOOPS: u32 = 1_000;

    #[test]
    fn entity_default() {
        let _ = env_logger::try_init();
        let now = std::time::Instant::now();
        let mut e = Entity::default();
        assert!(UUID_RE.is_match(e.id()));
        assert!(e.id().starts_with("entity-"));
        assert_eq!(UNNAMED, e.title());
        e.mp_mut().set_drain(-1.0);
        for _ in 0..LOOPS {
            // re-UUID is heavy, and it'd never be used in a loop like this in reality, but...:
            let old_id = e.id().to_string();
            e.re_uuid();
            assert_ne!(old_id.as_str(), e.id());
            let mut next_val = 100.0;
            e.mp_mut().set_curr(next_val);
            while next_val > 0.0 {
                next_val -= 1.0;
                if !e.tick() {
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
        let (w, c,p, _) = get_operational_mock_world().await;
        get_operational_mock_librarian!(c,w);
        get_operational_mock_life!(c,w);
        tokio::time::sleep(Duration::from_secs(3)).await;// let things stabilize in peace…
        let Ok(mob) = Entity::new("goblin").await else {
            panic!("Where'd the lil goblin go?!");
        };
        if let Err(e) = mob.save_bp().await {
            panic!("goblin fail: {e:?}");
        }
        let _ = c.out.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into() });
        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, LookCommand, "",s,c.out,w,p,|out:&str| out.contains("goblin is here"));
        p.write().await.config.show_id = true;
        p.write().await.access = Access::Builder;
        let _ = ctx!(state, LookCommand, "",s,c.out,w,p,|out:&str| out.contains("goblin-"));
    }
}
