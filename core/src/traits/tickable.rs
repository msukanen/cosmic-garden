//! General purpose [Tickable] trait…

use async_trait::async_trait;

#[async_trait]
pub trait Tickable {
    async fn tick(&mut self) -> bool;
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
