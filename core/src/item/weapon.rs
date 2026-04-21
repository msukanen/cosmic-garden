//! Weapons for everyone? Nah, but at least for players and (most of) the enemies…

use cosmic_garden_pm::{DescribableMut, IdentityMut, ItemizedMut, OwnedMut};
use serde::{Deserialize, Serialize};

use crate::{combat::Damager, item::{container::specs::StorageSpace, ownership::Owner}, mob::StatValue, string::Uuid, traits::Reflector};

/// Weapons tend to come in various sizes, which carries to how they're used + other specs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WeaponSize {
    /// From needles to pocket knives…
    Tiny,
    /// Small knives upto long daggers.
    Small,
    /// Generic swords and stuff.
    Medium,
    /// Two-handed things.
    Large,
    /// The largest two-handed things (including very large/long polearms).
    Huge,
}
impl Default for WeaponSize {
    fn default() -> Self {
        Self::Medium
    }
}

/// Weapon specs…
#[derive(Debug, Clone, Serialize, Deserialize, IdentityMut, OwnedMut, ItemizedMut, DescribableMut)]
pub struct WeaponSpec {
    pub(crate) id: String,
    #[identity(title)]
    pub(crate) name: String,
    pub(crate) desc: String,
    pub(crate) owner: Owner,
    pub(crate) size: StorageSpace,
    /// Size/classification for combat math…
    pub(crate) weapon_size: WeaponSize,
    /// The weapon's base dmg.
    pub(crate) base_dmg: StatValue,
}

impl Damager for WeaponSpec {
    /// Get the dmg the weapon does. This involves some randomness…
    fn dmg(&self) -> StatValue {
        self.base_dmg
    }
}

impl Reflector for WeaponSpec {
    fn reflect(&self) -> Self {
        Self { id: self.id.re_uuid(), ..self.clone() }
    }

    fn deep_reflect(&self) -> Self {
        self.reflect()
    }
}
