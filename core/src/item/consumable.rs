//! Consumable matter. Usually food, but not always.
use std::{collections::HashMap, fmt::Display};

use cosmic_garden_pm::{DescribableMut, IdentityMut, ItemizedMut, OwnedMut};
use serde::{Deserialize, Serialize};

use crate::{item::{container::specs::StorageSpace, matter::{Matter, MatterState}, ownership::Owner}, mob::{StatType, StatValue, affect::{Affect, Affector}}, string::Uuid, traits::{Reflector, Tickable}};

/// Various nutrition types (plus not edible).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum EffectType {
    /// It's from McDonalds. Don't try eat it - opt for crayons instead.
    NotEdible,
    /// Healing (or damaging with negative `drain`) property.
    Heal { stat: StatType, drain: StatValue },
    MultiHeal { stat_n_drain: HashMap<StatType, StatValue> }
}

impl Default for EffectType {
    fn default() -> Self {
        Self::NotEdible
    }
}

impl Display for EffectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEdible => write!(f, "inedible"),
            Self::Heal { stat, drain } => write!(f, "Heal({} {:+.2})", stat, drain),
            Self::MultiHeal { stat_n_drain } =>
                write!(f, "MultiHeal({})",
                        stat_n_drain
                            .iter()
                            .map(|(stat, drain)| format!("{} {:+.2}", stat, drain))
                            .collect::<Vec<_>>()
                            .join(", "))
        }
    }
}

/// Consumable matter. Be it edible stuff or from McDonalds…
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, ItemizedMut, DescribableMut, OwnedMut)]
pub struct ConsumableMatter {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) owner: Owner,
    pub(crate) size: StorageSpace,
    pub(crate) nutrition: EffectType,
    pub(crate) desc: String,
    pub(crate) matter_state: MatterState,
    /// Number of remaining uses.
    /// 
    /// ∞ uses is *effective* anything:
    /// * with an absurd amount of uses left.
    /// * `None` as [uses]; technically same as ∞.
    pub(crate) uses: Option<usize>,
    /// How many ticks each use lasts, if any.
    pub(crate) affect_ticks: Option<usize>,
    pub(crate) rots_in_ticks: Option<usize>,
}

impl Matter for ConsumableMatter {
    fn matter_state(&self) -> MatterState {
        self.matter_state
    }
}

pub trait Consumable {
    fn uses(&self) -> Option<usize>;
    fn nutrition(&self) -> EffectType;
}

pub trait ConsumableMut: Consumable {
    fn uses_mut(&mut self) -> &mut Option<usize>;
    fn nutrition_mut(&mut self) -> &mut EffectType;
}

impl Consumable for ConsumableMatter {
    fn uses(&self) -> Option<usize> {
        self.uses
    }

    fn nutrition(&self) -> EffectType {
        self.nutrition.clone()
    }
}

impl ConsumableMut for ConsumableMatter {
    fn uses_mut(&mut self) -> &mut Option<usize> {
        &mut self.uses
    }

    fn nutrition_mut(&mut self) -> &mut EffectType {
        &mut self.nutrition
    }
}

impl Reflector for ConsumableMatter {
    fn reflect(&self) -> Self {
        Self { id: self.id.re_uuid(), ..self.clone() }
    }
    fn deep_reflect(&self) -> Self {
        self.reflect()
    }
}

impl Affector for ConsumableMatter {
    fn as_affect(&self) -> Option<Affect> {
        match self.nutrition {
            EffectType::Heal { .. } =>
                Some(Affect::Effect { kind: self.nutrition.clone(), remaining: self.affect_ticks.clone() }),
            _ => None
        }
    }
}

impl Tickable for ConsumableMatter {
    fn tick(&mut self) -> bool {
        if let Some(t) = &mut self.rots_in_ticks {
            *t = t.saturating_sub(1);
            #[cfg(debug_assertions)]{
                if *t==0 {log::debug!("{} rotten.", self.id)}
            }
            *t == 0
        } else {false}
    }
}
