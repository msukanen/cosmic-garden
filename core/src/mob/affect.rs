//! Affects on Mobs (incl. [Player]).

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{item::consumable::EffectType, identity::uniq::Uuid, traits::Tickable};

/// All sorts of affects from good to bad to something else…
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Affect {
    Effect { kind: EffectType, remaining: Option<usize> },
    DelayedEffect { kind: EffectType, remaining: Option<usize>, delay: Option<usize> },
    RushNCrash { kind: EffectType, remaining: Option<usize>, crash_kind: EffectType, delay: Option<usize>, crash_remain: Option<usize> },
    HardcorePending { remaining: Option<usize> },
    Expired,
}

impl Affect {
    pub fn hardcore_pending(&self) -> bool {
        matches!(self, Self::HardcorePending { .. })
    }
}

#[async_trait]
impl Tickable for Affect {
    async fn tick(&mut self) -> bool {
        match self {
            // Decays:
            Self::Effect { remaining: Some(1),.. } => {
                *self = Self::Expired;
                true
            }
            
            Self::HardcorePending { remaining: Some(1)} => {
                *self = Self::Expired;
                false
            }
           
            Self::DelayedEffect { delay: Some(1), kind, remaining } =>{
                *self = Self::Effect {
                            kind: kind.clone(),
                            remaining: remaining.clone()
                        };
                true
            }
            
            Self::RushNCrash { remaining: Some(1), crash_kind, delay, crash_remain, .. } => {
                *self = Self::DelayedEffect {
                            kind: crash_kind.clone(),
                            remaining: crash_remain.clone(),
                            delay: delay.clone()
                        };
                true
            }
            
            // Tick-tock goes the clock…
            Self::DelayedEffect { delay: Some(x), ..}    |
            Self::DelayedEffect { remaining: Some(x),..} |
            Self::Effect { remaining: Some(x),.. }       |
            Self::HardcorePending { remaining: Some(x) } |
            Self::RushNCrash { remaining: Some(x),.. }   => { *x = x.saturating_sub(1); true },

            // Placeholder(s)…
            Self::Expired => false,

            _ => unreachable!("Ouch!")
        }
    }
}

impl Affect {
    pub fn expired(&self) -> bool {
        match self {
            Self::Effect { remaining: None,.. } => false,
            Self::Effect { remaining: Some(x),.. } => *x == 0,
            Self::DelayedEffect { .. }   |        // Delayed decays into Effect
            Self::HardcorePending { .. } |        // H.P. decays to Expired
            Self::RushNCrash { .. }      => false,// RNC will decay into DelayedEffect
            
            Self::Expired => true,
        }
    }

    pub fn dormant(&self) -> bool {
        match self {
            Self::Effect { remaining: Some(x),.. } => *x == 0,
            Self::DelayedEffect { delay: Some(_),.. } |
            Self::Expired => true,
            _ => false
        }
    }
}

pub trait Affector {
    fn as_affect(&self) -> Option<Affect>;
}

/// Stack `affect` in stash if/when possible/needed.
pub fn stack_affect(item: &str, affect: &Affect, stash: &mut HashMap<String, Affect>) {
    let nouuid = item.no_uuid();
    let mut stashed = false;
    if let Some(existing) = stash.get_mut(nouuid) {
        match (existing, affect) {
            (
                Affect::Effect { kind: ek, remaining: ex_rem },
                Affect::Effect { kind: nk, remaining: new_rem }
            ) if ek == nk => {
                *ex_rem = match (*ex_rem, *new_rem) {
                    (Some(a), Some(b)) => Some(a+b),
                    (_, None) => None,
                    (None, Some(_)) => None
                };
                stashed = true;
            },
            _ => {}// fall-thro
        }
    }
    
    if !stashed {
        stash.insert(nouuid.to_string(), affect.clone());
    }
}

#[cfg(test)]
mod affect_tests {
    use std::collections::HashMap;

    use crate::{item::consumable::EffectType, mob::{StatType, affect::{Affect, stack_affect}}, traits::Tickable};

    #[test]
    fn affect_stacking() {
        let a = Affect::Effect { kind: EffectType::Heal { stat: StatType::HP, drain: 1.0 }, remaining: Some(3) };
        let b = Affect::Effect { kind: EffectType::Heal { stat: StatType::HP, drain: 1.0 }, remaining: Some(3) };
        let mut stash = HashMap::new();
        stack_affect("item", &a, &mut stash);
        assert!(stash.contains_key("item"));
        stack_affect("item", &b, &mut stash);
        assert!(stash.contains_key("item"));
        let Some(Affect::Effect { remaining, .. }) = stash.get("item") else {
            panic!("Where'd it go?");
        };
        assert_eq!(Some(6), *remaining);
    }

    #[tokio::test]
    async fn affect_decay() {
        let _ = env_logger::try_init();
        let mut r = Affect::RushNCrash {
            kind: EffectType::NotEdible,
            remaining: Some(3),
            crash_kind: EffectType::NotEdible,
            delay: Some(3),
            crash_remain: Some(3)
        };// this should be 9 ticks lifetime before .expired()

        log::debug!("r = {r:?}");
        assert!(!r.dormant());
        assert!(!r.expired());
        for _ in 0..2 {
            r.tick().await;
            log::debug!(">   {r:?}");
            assert!(!r.dormant());
            assert!(!r.expired());
        }
        r.tick().await;
        log::debug!("r = {r:?}");
        assert!(r.dormant());
        assert!(!r.expired());
        assert!(matches!(r, Affect::DelayedEffect {..}));
        for _ in 0..2 {
            r.tick().await;
            log::debug!(">   {r:?}");
            assert_eq!(true, r.dormant());
            assert_eq!(false, r.expired());
        }
        r.tick().await;
        log::debug!("r = {r:?}");
        assert!(!r.dormant());
        assert!(!r.expired());
        assert!(matches!(r, Affect::Effect {..}));
        for _ in 0..2 {
            r.tick().await;
            log::debug!(">   {r:?}");
            assert_eq!(false, r.dormant());
            assert_eq!(false, r.expired());
        }
        r.tick().await;
        assert!(matches!(r, Affect::Expired));
        assert_eq!(true, r.dormant());
        assert_eq!(true, r.expired());
        log::debug!("r = {r:?}");
    }
}