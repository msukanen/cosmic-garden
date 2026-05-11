//! Mob related stuff…
pub mod affect;
pub mod ai;     pub use ai::Ai;
pub mod core;
pub mod faction;
pub mod gender;

pub use gender::*;
pub mod spawn_lib;
pub mod stat;   pub use stat::*;
pub mod traits;

/// Entity arc type.
pub type EntityArc = std::sync::Arc<tokio::sync::RwLock<core::Entity>>;
impl Into<EntityArc> for core::Entity {
    fn into(self) -> EntityArc {
        std::sync::Arc::new(tokio::sync::RwLock::new(self))
    }
}
/// Entity weak arc type.
pub type EntityWeak = std::sync::Weak<tokio::sync::RwLock<core::Entity>>;
