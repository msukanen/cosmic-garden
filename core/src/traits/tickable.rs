//! General purpose [Tickable] trait…

use async_trait::async_trait;

use crate::{combat::CombatantMut, mob::core::Entity};

#[async_trait]
pub trait Tickable {
    async fn tick(&mut self) -> bool;
}

#[async_trait]
impl Tickable for Entity {
    async fn tick(&mut self) -> bool {
        let hp = self.hp_mut().tick().await;
        let mp = self.mp_mut().tick().await;
        let sn = self.sn_mut().tick().await;
        let san = self.san_mut().tick().await;
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
