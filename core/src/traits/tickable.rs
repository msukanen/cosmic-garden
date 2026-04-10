//! General purpose [Tickable] trait…

use crate::mob::traits::MobMut;

pub trait Tickable {
    fn tick(&mut self) -> bool;
}

impl <T: MobMut> Tickable for T {
    fn tick(&mut self) -> bool {
        let hp = self.hp_mut().tick();
        let mp = self.mp_mut().tick();
        let sn = self.sn_mut().tick();
        let san = self.san_mut().tick();
        hp || mp || sn || san
    }
}
