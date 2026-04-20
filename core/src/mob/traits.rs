//! Mob specific traits.

use crate::{combat::{Combatant, CombatantMut}, identity::IdentityQuery, item::weapon::WeaponSize};

/// A trait for anything "mobile".
pub trait Mob : IdentityQuery + Combatant {
    /// Get maximum weapon size the combatant can wield.
    fn max_weapon_size(&self) -> WeaponSize;
}

/// Mutable variant of [Mob].
pub trait MobMut : Mob + CombatantMut {
}
