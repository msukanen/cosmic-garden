//! Mob specific traits.

use crate::{combat::{Combatant, CombatantMut}, identity::IdentityQuery, mob::{Stat, StatError}};

/// A trait for anything "mobile".
pub trait Mob : IdentityQuery + Combatant {
    /// Get current HP (health points)
    fn hp<'a>(&'a self) -> &'a Stat;
    /// Get current MP (mental points)
    fn mp<'a>(&'a self) -> &'a Stat;
    /// Get current SN (strain points)
    fn sn<'a>(&'a self) -> &'a Stat;
    /// Get current SAN (sanity points)
    fn san<'a>(&'a self) -> &'a Stat;

    /// Get current Str(ength)
    fn str<'a>(&'a self) -> &'a Stat;
    /// Get current Nim(bleness)
    fn nim<'a>(&'a self) -> &'a Stat;
    /// Get current Brn(iness)
    fn brn<'a>(&'a self) -> &'a Stat;

    /// Is the Mob unconscious?
    fn is_unconscious(&self) -> Result<bool, StatError> {
        match (
            self.hp().is_unconscious(),
            self.mp().is_unconscious(),
            self.sn().is_unconscious(),
            self.san().is_unconscious(),
        ) {
            (Ok(true),..)  |
            (_,Ok(true),..)|
            (_,_,Ok(true),..) |
            (_,_,_,Ok(true)) => Ok(true),
            _ => Ok(false)
        }
    }

    /// Is the [Mob] dead?
    fn is_dead(&self) -> bool { self.hp().is_dead().ok().unwrap() }
}

/// Mutable variant of [Mob].
pub trait MobMut : Mob + CombatantMut {
    /// Get mutable HP.
    fn hp_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable MP.
    fn mp_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable SN.
    fn sn_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable San.
    fn san_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable Str.
    fn str_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable Brn.
    fn brn_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable Nim.
    fn nim_mut<'a>(&'a mut self) -> &'a mut Stat;
}
