//! Storage trait(or)ing…

use std::fmt::Display;

use crate::{identity::IdentityQuery, item::{Item, container::specs::StorageSpace}};

/// Various storage related errors.
/// 
/// Note: all the error codes except the **Q**(uery)-variants will carry the [Item] along. No matter wasted…
/// Generally the **Q**-variants are returned by e.g. [Storage::can_hold] and alike.
#[derive(Debug)]
pub enum StorageError {
    /// Target isn't even a container.
    NotContainerQ,
    /// Target isn't even a container.
    NotContainer(Item),

    /// Can't fit the [Item]…
    NoSpaceQ,
    /// Can't fit the [Item]…
    NoSpace(Item),

    /// Right — a pouch cannot hold a backpack, no matter how you try to compress the poor bag…
    InvalidHierarchyQ,
    /// Right — a pouch cannot hold a backpack, no matter how you try to compress the poor bag…
    InvalidHierarchy(Item),
}

impl Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHierarchyQ => write!(f, "Invalid container hierarchy attempt…"),
            Self::InvalidHierarchy(i) => write!(f, "Hierarchy error: {} cannot be fitted within.", i.id()),
            
            Self::NoSpaceQ => write!(f, "No space."),
            Self::NoSpace(i) => write!(f, "No space for {}.", i.id()),

            Self::NotContainerQ => write!(f, "That doesn't fit there!"),
            Self::NotContainer(i) => write!(f, "Not a container, ergo {} cannot be inserted.", i.id()),
        }
    }
}

impl std::error::Error for StorageError {}

pub trait Storage {
    /// Check how much space there is left in the container.
    fn space(&self) -> StorageSpace {0}
    /// Check how much the container can hold in total.
    fn max_space(&self) -> StorageSpace {0}
    /// Check how much space the container + its contents require.
    fn required_space(&self) -> StorageSpace {1}
    /// Check whether the container can hold on to [`item`][Item].
    fn can_hold(&self, _item: &Item) -> Result<bool, StorageError> {Err(StorageError::NotContainerQ)}
    /// Try insert an [`item`][Item].
    #[must_use = "Item is contained within StorageError"]
    fn try_insert(&mut self, _item: Item) -> Result<(), StorageError>;
    /// See if `id` is contained.
    fn contains(&self, id: &str) -> bool;
    /// Eyeball an [Item] of `id`, if it happens to be contained.
    fn peek_at(&self, id: &str) -> Option<&Item>;
    /// Very literally yank out `id`, if present.
    #[must_use = "Item taken out will require handling"]
    fn take(&mut self, id: &str) -> Option<Item>;
    #[must_use = "Item taken out will require handling"]
    fn take_by_name(&mut self, id: &str) -> Option<Item>;
    /// Find item ID by `name` (or title, UUID, etc.).
    fn find_id_by_name(&self, name: &str) -> Option<String>;
    /// Eject all the contents!
    fn eject_all(&mut self) -> Option<Vec<Item>>;
}

pub trait StorageMut {
    /// Set maximum space.
    /// 
    /// Note that this does not succeed if:
    /// * there's more content than new space allows.
    fn set_max_space(&mut self, sz: StorageSpace) -> bool;
}

impl StorageError {
    pub fn extract_item(self) -> Option<Item> {
        match self {
            Self::InvalidHierarchy(i) |
            Self::NoSpace(i) |
            Self::NotContainer(i) => i.into(),
            
            Self::InvalidHierarchyQ|
            Self::NoSpaceQ|
            Self::NotContainerQ => None
        }
    }
}
