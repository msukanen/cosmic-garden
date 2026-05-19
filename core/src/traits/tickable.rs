//! General purpose [Tickable] trait…

use async_trait::async_trait;

use crate::{item::consumable::EffectType, mob::ai::*, room::environ::{EnvironmentEvent, SpecialEnvironment, Terrain}};

/// A trait for anything with (un)steady tick rate.
#[async_trait]
pub trait Tickable {
    fn tick(&mut self, curr_tick: usize, room_env: SpecialEnvironment, room_terrain: Option<Terrain>) -> Option<Vec<TickMeaning>>;
    #[inline]
    fn should_pulse(&self, curr_tick: usize, tick_id: usize, modulo: usize) -> bool {
        should_pulse(curr_tick.wrapping_add(tick_id), modulo)
    }
}

/// Should something pulse?
#[inline] const fn should_pulse(tick_id: usize, divisor: usize) -> bool {
    tick_id % divisor == 0
}

/// - "What it means, what it means?"
/// 
/// More meaning to the tick results, of course.
#[derive(Debug, Clone)]
pub enum TickMeaning {
    General,
    AffectPossessor { kind: EffectType },
    EnvironmentEffect { what: EnvironmentEvent },
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
