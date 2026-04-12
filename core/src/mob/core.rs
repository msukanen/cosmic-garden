//! Mob core.

use cosmic_garden_pm::{IdentityMut, MobMut};
use serde::{Deserialize, Serialize};

use crate::{identity::{IdentityMut, IdentityQuery}, mob::*, string::{UNNAMED, Uuid, as_id_with_uuid}};

#[derive(Debug, Deserialize, Serialize, IdentityMut, MobMut)]
pub struct Entity {
    id: String,
    #[identity(title)]
    name: String,
    hp: Stat,
    mp: Stat,
    san: Stat,
    sn: Stat,
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
        }
    }
}

impl Entity {
    pub fn re_uuid(&mut self) {
        *self.id_mut() = self.id().re_uuid()
    }
}

#[cfg(test)]
mod entity_tests {
    use crate::{identity::IdentityQuery, mob::{core::Entity, traits::{Mob, MobMut}}, string::{UNNAMED, UUID_RE}, traits::Tickable};

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
}
