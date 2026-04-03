//! Storage trait(or)ing…

use crate::item::{Item, container::specs::StorageSpace};

/// Various storage related errors.
/// 
/// Note: all the error codes will carry the [Item] along. No matter wasted…
#[derive(Debug)]
pub enum StorageError {
    /// Target isn't even a container.
    NotContainer(Item),
    /// Can't fit the [Item]…
    NoSpace(Item),
    /// Right — a pouch cannot hold a backpack, no matter how you try to compress the poor bag…
    InvalidHierarchy(Item),
}

pub trait Storage {
    /// Check how much space there is left in the container.
    fn space(&self) -> StorageSpace {0}
    /// Check how much the container can hold in total.
    fn max_space(&self) -> StorageSpace {0}
    /// Check how much space the container + its contents require.
    fn required_space(&self) -> StorageSpace {1}
    /// Check whether the container can hold on to `_item`.
    fn can_hold(&self, _item: &Item) -> bool {false}
}
