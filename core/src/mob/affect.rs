//! Affects on Mobs (incl. [Player]).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{item::consumable::NutritionType, string::Uuid, traits::Tickable};

/// All sorts of affects from good to bad to something else…
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Affect {
    Nutrition { kind: NutritionType, remaining: Option<usize> },
    RushNCrash { kind: NutritionType, remaining: Option<usize>, delay: Option<usize> },
    Other,
}

impl Tickable for Affect {
    fn tick(&mut self) -> bool {
        match self {
            Self::Nutrition { remaining: None,.. }  |
            Self::RushNCrash { remaining: None,.. } => true,
            Self::RushNCrash { remaining: Some(0),.. } |
            Self::Nutrition { remaining: Some(0),.. }  => true,

            Self::RushNCrash { remaining: Some(x), delay: None,.. } |
            Self::Nutrition { remaining: Some(x),.. }
                => { if *x > 0 {*x = *x - 1;}; true },

            Self::RushNCrash { delay: Some(x), kind, remaining }
                => {
                    *x = x.saturating_sub(1);
                    if *x == 0 {
                        *self = Affect::RushNCrash { kind: kind.clone(), remaining: remaining.clone(), delay: None };
                    }
                    false
                },
            
            Self::Other => false,
        }
    }
}

impl Affect {
    pub fn expired(&self) -> bool {
        match self {
            Self::Nutrition { remaining: None,.. }  => false,
            Self::Nutrition { remaining: Some(x),.. } |
            Self::RushNCrash { remaining: Some(x),.. }
                => *x == 0,
            Self::Other => true,
            _ => true
        }
    }

    pub fn dormant(&self) -> bool {
        match self {
            Self::Other => true,
            Self::RushNCrash { delay: Some(x),.. } if *x > 0 => true,
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
    if let Some(existing) = stash.get_mut(&nouuid) {
        match (existing, affect) {
            (
                Affect::Nutrition { kind: ek, remaining: ex_rem },
                Affect::Nutrition { kind: nk, remaining: new_rem }
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
        stash.insert(nouuid, affect.clone());
    }
}

#[cfg(test)]
mod affect_tests {
    use std::collections::HashMap;

    use crate::{item::consumable::NutritionType, mob::{StatType, affect::{Affect, stack_affect}}};

    #[test]
    fn affect_stacking() {
        let a = Affect::Nutrition { kind: NutritionType::Heal { stat: StatType::HP, drain: 1.0 }, remaining: Some(3) };
        let b = Affect::Nutrition { kind: NutritionType::Heal { stat: StatType::HP, drain: 1.0 }, remaining: Some(3) };
        let mut stash = HashMap::new();
        stack_affect("item", &a, &mut stash);
        assert!(stash.contains_key("item"));
        stack_affect("item", &b, &mut stash);
        assert!(stash.contains_key("item"));
        let Some(Affect::Nutrition { kind, remaining }) = stash.get("item") else {
            panic!("Where'd it go?");
        };
        assert_eq!(Some(6), *remaining);
    }
}