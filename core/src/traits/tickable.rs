//! General purpose [Tickable] trait…

use crate::mob::{core::Entity, traits::MobMut};

pub trait Tickable {
    fn tick(&mut self) -> bool;
}

impl Tickable for Entity {
    fn tick(&mut self) -> bool {
        let hp = self.hp_mut().tick();
        let mp = self.mp_mut().tick();
        let sn = self.sn_mut().tick();
        let san = self.san_mut().tick();
        hp || mp || sn || san
    }
}

/// - "What it means, what it means?"
/// 
/// More meaning to the tick results, of course.
#[derive(Debug, Clone, Copy)]
pub enum TickMeaning {
    Nothing,
    General,
    StatShift,
}
