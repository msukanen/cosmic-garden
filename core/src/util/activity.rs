//! Some player activity related things.

use std::ops::AddAssign;

#[derive(Debug, Clone)]
pub enum ActionWeight {
    Drain,
    Move,
    Xp,
    Level,
    ItemTransfer { count: usize }
}

impl From<ActionWeight> for usize {
    fn from(value: ActionWeight) -> Self {
        match value {
            ActionWeight::Drain => 1,
            ActionWeight::ItemTransfer { count } => count * 8,
            ActionWeight::Level => 1_000,
            ActionWeight::Move => 5,
            ActionWeight::Xp => 2,
        }
    }
}

impl AddAssign<ActionWeight> for usize {
    fn add_assign(&mut self, rhs: ActionWeight) {
        *self = *self + usize::from(rhs)
    }
}
