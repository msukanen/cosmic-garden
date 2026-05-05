//! Mob related stuff…
pub mod affect;
pub mod core;
pub mod faction;
pub mod gender; use std::sync::{Arc, Weak};

pub use gender::*;
pub mod spawn_lib;
pub mod stat;   pub use stat::*;
use tokio::sync::RwLock;
pub mod traits;

/// Entity arc type.
pub type EntityArc = Arc<RwLock<core::Entity>>;
impl Into<EntityArc> for core::Entity {
    fn into(self) -> EntityArc {
        std::sync::Arc::new(tokio::sync::RwLock::new(self))
    }
}
/// Entity weak arc type.
pub type EntityWeak = Weak<RwLock<core::Entity>>;
