//! Weapons for everyone? Nah, but at least for players and (most of) the enemies…

use std::{fmt::Display, ops::{Div, Mul}};

use cosmic_garden_pm::{DescribableMut, IdentityMut, ItemizedMut, OwnedMut};
use serde::{Deserialize, Serialize};

use crate::{combat::Damager, r#const::{HUGE_ITEM, LARGE_ITEM, MEDIUM_ITEM, SIZE_BALANCE, SMALL_ITEM, TINY_ITEM}, item::{container::specs::StorageSpace, ownership::Owner}, mob::StatValue, uuid::Uuid, traits::Reflector};

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

impl WeaponSize {
    /// How in [StorageSpace] units the weapon takes.
    pub fn required_space(&self) -> StorageSpace {
        match self {
            Self::Huge => HUGE_ITEM,
            Self::Large => LARGE_ITEM,
            Self::Medium => MEDIUM_ITEM,
            Self::Small => SMALL_ITEM,
            Self::Tiny => TINY_ITEM,
        }
    }
}

impl TryFrom<&str> for WeaponSize {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value.to_lowercase().chars().nth(0) {
            Some('t') => Self::Tiny,
            Some('s') => Self::Small,
            Some('m') => Self::Medium,
            Some('l') => Self::Large,
            Some('h') => Self::Huge,
            _ => return Err(format!("'{value}' is not recognized as weapon category, use: H, L, M, S, T"))
        })
    }
}

impl Display for WeaponSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Huge => "huge",
            Self::Large => "large",
            Self::Medium => "medium",
            Self::Small => "small",
            Self::Tiny => "tiny",
        })
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
    pub(crate) weapon_size: WeaponSize, // gives fixed minimum size for the weapon, with no upper limit though.
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

impl Div<StatValue> for WeaponSize {
    type Output = StatValue;
    fn div(self, rhs: StatValue) -> Self::Output {
        (self.required_space() / SIZE_BALANCE as StorageSpace) as StatValue / rhs
    }
}

impl Mul<StatValue> for WeaponSize {
    type Output = StatValue;
    fn mul(self, rhs: StatValue) -> Self::Output {
        (self.required_space() / SIZE_BALANCE as StorageSpace) as StatValue / rhs
    }
}

impl From<&WeaponSize> for i8 {
    fn from(value: &WeaponSize) -> Self {
        match value {
            WeaponSize::Tiny => -2,
            WeaponSize::Small => -1,
            WeaponSize::Medium => 0,
            WeaponSize::Large => 1,
            WeaponSize::Huge => 2,
        }
    }
}

pub const fn str_based_dmg_mul(strn: StatValue, npc: bool) -> f32 {
    if npc {
        strn / 75.0
    } else {
        strn / 50.0
    }
}
