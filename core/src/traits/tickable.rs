//! General purpose [Tickable] trait…

use async_trait::async_trait;

use crate::{item::consumable::EffectType, mob::ai::*, room::environ::{SpecialEnvironment, Terrain}};

#[async_trait]
pub trait Tickable {
    fn tick(&mut self, curr_tick: usize, room_env: SpecialEnvironment, room_terrain: Option<Terrain>) -> Option<Vec<TickMeaning>>;
}

/// - "What it means, what it means?"
/// 
/// More meaning to the tick results, of course.
#[derive(Debug, Clone)]
pub enum TickMeaning {
    General,
    AffectPossessor { kind: EffectType },
    EnvironmentEffect,// TODO bubble up environment effects
    AiStateChange { maybe_state: Option<AiState>, maybe_mental_state: Option<AiMentalState>, maybe_action: Option<AiAction> },
}

#[macro_export]
macro_rules! general_tick {
    () => {
        Some(vec![crate::traits::tickable::TickMeaning::General])
    };
}

#[macro_export]
macro_rules! single_tick_meaning {
    ($meaning:expr) => {
        Some(vec![$meaning])
    };
}
