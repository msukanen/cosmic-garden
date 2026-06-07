//! Quest system.

use cosmic_garden_pm::IdentityMut;

pub mod objective;
pub use objective::*;

#[derive(Debug, Clone, IdentityMut)]
pub struct Quest {
    id: String,
    title: String,

    objective: Objective,
}
