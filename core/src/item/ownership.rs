//! [Item][crate::item::Item] ownership tracking.

use cosmic_garden_pm::OwnedMut;
use serde::{Deserialize, Serialize};

use crate::identity::IdError;

/// [Item][crate::item::Item] ownership queries.
pub trait Owned {
    /// True owner's ID, if any.
    fn owner(&self) -> Option<String>;
    
    /// Last user's ID, if any.
    fn last_user(&self) -> Option<String>;

    /// Source of the [Item][crate::item::Item].
    fn source(&self) -> ItemSource;
}

/// [Item][crate::item::Item] ownership mutation.
pub trait OwnedMut {
    /// Change the [Item][crate::item::Item] owner.
    fn change_owner(&mut self, new_id: &str);
    /// Change last user of [Item][crate::item::Item].
    fn set_last_user(&mut self, new_id: &str) -> Result<(), IdError>;
    /// Set [ItemSource] of [Item][crate::item::Item].
    fn set_source(&mut self, of: &str, by: &str, new_source: ItemSource) -> Result<(), ItemSourceError>;
}

#[derive(Debug, Clone, Deserialize, Serialize, OwnedMut)]
pub struct Owner {
    owner_id: Option<String>,
    last_user_id: Option<String>,
    source: ItemSource,
}

#[derive(Debug, Clone)]
pub enum ItemSourceError {
    Rejected,
}

impl Owner {
    /// No specific owner (yet) — straight from [BlueprintLibrary][crate::item::library::BlueprintLibrary].
    pub fn blueprint() -> Self {
        Self { owner_id: None, last_user_id: None, source: ItemSource::Blueprint }
    }

    /// No specific owner (yet) — System-generated. Usually for constructs like player/room inventories.
    pub fn no_one() -> Self {
        Self { owner_id: None, last_user_id: None, source: ItemSource::System }
    }
}

/// Where did the [Item] originate from?
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ItemSource {
    /// Monster loot or vendor stuff…
    Spawn {
        /// Monster drop or otherwise?
        monster_drop: bool,
    },
    /// Crafted by some [Player].
    // TODO crafting profession
    PlayerCrafted {
        id: String,
        name: String
    },
    /// Admin hand-out.
    Admin { id: String },
    /// Quest item.
    Quest,
    /// Fresh out of BlueprintLibrary (for items that are not yet
    /// actualized into gameplay with 'weave' or otherwise and just
    /// for now sit in a builder's iedit_buffer or in the library itself).
    Blueprint,
    /// System - not specifically from anyone or anywhere, but for basic features like player inventory, rooms, etc.
    System
}
