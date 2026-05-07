//! General purpose [Tickable] trait…

use async_trait::async_trait;

use crate::item::consumable::EffectType;

#[async_trait]
pub trait Tickable {
    fn tick(&mut self) -> Option<Vec<TickMeaning>>;
}

/// - "What it means, what it means?"
/// 
/// More meaning to the tick results, of course.
#[derive(Debug, Clone)]
pub enum TickMeaning {
    General,
    AffectPossessor { kind: EffectType },
    EnvironmentEffect,// TODO environment effects
}

#[macro_export]
macro_rules! general_tick {
    () => {
        Some(vec![crate::traits::tickable::TickMeaning::General])
    };
}
