//! Mob specific traits.

use crate::{identity::IdentityQuery, mob::{Stat, StatError}};

pub trait Mob : IdentityQuery {
    fn hp<'a>(&'a self) -> &'a Stat;
    fn mp<'a>(&'a self) -> &'a Stat;
    fn sn<'a>(&'a self) -> &'a Stat;
    fn san<'a>(&'a self) -> &'a Stat;

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

    fn is_dead(&self) -> Result<bool, StatError> { self.hp().is_dead() }
}

pub trait MobMut : Mob {
    fn hp_mut<'a>(&'a mut self) -> &'a mut Stat;
    fn mp_mut<'a>(&'a mut self) -> &'a mut Stat;
    fn sn_mut<'a>(&'a mut self) -> &'a mut Stat;
    fn san_mut<'a>(&'a mut self) -> &'a mut Stat;
}
