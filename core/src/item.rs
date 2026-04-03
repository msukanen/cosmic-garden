//! Items dwell here…

use cosmic_garden_pm::{IdentityMut, Itemized};
use serde::{Deserialize, Serialize};

use crate::item::container::{Storage, StorageError, specs::StorageSpace, variants::ContainerVariant};

pub mod owner;
pub mod container;
pub mod key;
pub mod tool;
pub mod weapon;

pub trait Itemized {
    fn size(&self) -> StorageSpace;
}

#[derive(Debug, Deserialize, Serialize, IdentityMut)]
pub struct X {id:String,title:String}
impl Itemized for X {
    fn size(&self) -> StorageSpace {
        1
    }
}

#[derive(Debug, Deserialize, Serialize, IdentityMut, Itemized)]
/// Root [Item] types.
pub enum Item {
    Container(ContainerVariant),
    Weapon(X),
    Tool(X),
    Key(X),
}

impl Storage for Item {
    fn can_hold(&self, item: &Item) -> Result<bool, StorageError> {
        match self {
            Self::Container(c) => c.can_hold(item),
            _ => Ok(false)
        }
    }

    fn max_space(&self) -> StorageSpace {
        match self {
            Self::Container(c) => c.max_space(),
            _ => 0
        }
    }

    fn required_space(&self) -> StorageSpace {
        match self {
            Self::Container(c) => c.required_space(),
            _ => self.size()
        }
    }

    fn space(&self) -> StorageSpace {
        match self {
            Self::Container(c) => c.space(),
            _ => 0
        }
    }

    fn try_insert(&mut self, item: Item) -> Result<(), container::StorageError> {
        match self {
            Self::Container(c) => c.try_insert(item),
            _ => Err(container::StorageError::NotContainer(item))
        }
    }
}
