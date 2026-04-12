//! Consumable matter. Usually food, but not always.
use std::fmt::Display;

use cosmic_garden_pm::{DescribableMut, IdentityMut, ItemizedMut};
use serde::{Deserialize, Serialize};

use crate::{item::container::specs::StorageSpace, mob::{StatType, StatValue, affect::{Affect, Affector}}, string::Uuid, traits::Reflector};

/// Various nutrition types (plus not edible).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum NutritionType {
    /// It's from McDonalds. Don't try eat it - opt for crayons instead.
    NotEdible,
    /// Healing (or damaging with negative `drain`) property.
    Heal { stat: StatType, drain: StatValue },
}

impl Display for NutritionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEdible => write!(f, "inedible"),
            Self::Heal { stat, drain } => write!(f, "Heal({} {:+.2})", stat, drain)
        }
    }
}

/// Consumable matter. Be it edible stuff or from McDonalds…
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, ItemizedMut, DescribableMut)]
pub struct ConsumableMatter {
    pub id: String,
    pub title: String,
    pub size: StorageSpace,
    pub nutrition: NutritionType,
    pub desc: String,
    /// Number of remaining uses.
    /// 
    /// ∞ uses is *effective* anything:
    /// * with an absurd amount of uses left.
    /// * `None` as [uses].
    pub uses: Option<usize>,
    /// How many ticks each use lasts, if any.
    pub affect_ticks: Option<usize>,
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
            NutritionType::Heal { .. } =>
                Some(Affect::Nutrition { kind: self.nutrition.clone(), remaining: self.affect_ticks.clone() }),
            _ => None
        }
    }
}
