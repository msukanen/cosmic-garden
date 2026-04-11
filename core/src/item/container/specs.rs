//! Container specs themselves.

use std::{cmp::Ordering, collections::HashMap};

use cosmic_garden_pm::{IdentityMut, Itemized};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{Item, Itemized as _, container::{Storage, StorageError, variants::ContainerVariant}}, string::{UNNAMED, Uuid}, traits::Reflector};

pub type StorageSpace = u16;

lazy_static! {
    pub(super) static ref DEFAULT_BACKPACK_SPEC: ContainerSpec = ContainerSpec {
        id: "backpack".with_uuid(),
        name: "backpack".into(),
        contents: HashMap::new(),
        max_space: 30,
        size: 2,
    };

    pub(super) static ref DEFAULT_POUCH_SPEC: ContainerSpec = ContainerSpec {
        id: "pouch".with_uuid(),
        name: "pouch".into(),
        contents: HashMap::new(),
        max_space: 10,
        size: 1,
    };

    pub(super) static ref DEFAULT_PLR_INV_SPEC: ContainerSpec = ContainerSpec {
        id: "player-inventory".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 50,
        size: 0,
    };

    pub(super) static ref DEFAULT_ROOM_SPACE_SPEC: ContainerSpec = ContainerSpec {
        id: "room-space".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 1_000,
        size: 0,
    };
}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, Itemized)]
pub struct ContainerSpec {
    id: String,
    #[identity(title)]
    name: String,
    contents: HashMap<String, Item>,
    max_space: StorageSpace,
    size: StorageSpace,
}

impl From<&ContainerSpec> for ContainerSpec {
    fn from(value: &ContainerSpec) -> Self {
        Self {
            id: value.id().re_uuid(),
            name: value.name.clone(),
            contents: HashMap::new(),
            max_space: value.max_space,
            size: value.size,
        }
    }
}

impl Reflector for ContainerSpec {
    fn reflect(&self) -> Self {
        let mut r = Self::from(self);
        for x in &self.contents {
            let refl = x.1.reflect();
            r.contents.insert(refl.id().into(), refl);
        }
        r
    }
}

impl ContainerSpec {
    fn contents_size(&self) -> StorageSpace {
        let mut sz = 0;
        for x in self.contents.values() {
            sz += x.required_space()
        }
        sz
    }
}

impl PartialEq<ContainerSpec> for ContainerSpec {
    fn eq(&self, other: &ContainerSpec) -> bool {
        self.max_space == other.max_space
    }
}

impl PartialEq<Item> for ContainerSpec {
    fn eq(&self, other: &Item) -> bool {
        match other {
            Item::Container(v) => match v {
                ContainerVariant::Backpack(v)|
                ContainerVariant::PlayerInventory(v) |
                ContainerVariant::Pouch(v) |
                ContainerVariant::Room(v)  => v.eq(self)
            },
            _ => false
        }
    }
}

impl PartialOrd<Item> for ContainerSpec {
    fn partial_cmp(&self, other: &Item) -> Option<Ordering> {
        match other {
            Item::Container(v) => match v {
                ContainerVariant::Backpack(v) |
                ContainerVariant::PlayerInventory(v) |
                ContainerVariant::Pouch(v) |
                ContainerVariant::Room(v) => self.partial_cmp(v),
            },
            _ => Some(Ordering::Greater)
        }
    }
}

impl PartialOrd<ContainerSpec> for ContainerSpec {
    fn partial_cmp(&self, other: &ContainerSpec) -> Option<Ordering> {
        self.max_space.partial_cmp(&other.max_space)
    }
}

impl Storage for ContainerSpec {
    fn max_space(&self) -> StorageSpace {
        self.max_space
    }
    
    fn required_space(&self) -> StorageSpace {
        self.size() + self.contents_size()
    }
    
    fn space(&self) -> StorageSpace {
        self.max_space - self.contents_size()
    }

    fn can_hold(&self, item: &Item) -> Result<bool, StorageError> {
        if self < item {
            return Err(StorageError::InvalidHierarchyQ);
        }
        let ok = item.required_space() as usize
            + self.contents_size() as usize
            <= self.max_space as usize;
        if ok {
            Ok(true)
        } else {
            Err(StorageError::NoSpaceQ)
        }
    }

    fn try_insert(&mut self, item: Item) -> Result<(), StorageError> {
        if let Err(e) = self.can_hold(&item) {
            // map q-variants into matter-holders
            return Err(match e {
                StorageError::NoSpaceQ => StorageError::NoSpace(item),
                StorageError::InvalidHierarchyQ => StorageError::InvalidHierarchy(item),
                StorageError::NotContainerQ => StorageError::NotContainer(item),
                _ => e
            });
        }
        self.contents.insert(item.id().into(), item);
        Ok(())
    }

    fn contains(&self, id: &str) -> bool {
        if self.max_space < 1 { return false; }
        for c in self.contents.values() {
            if c.id() == id || c.contains(id) { return true; }
        }
        true
    }

    fn peek_at(&self, id: &str) -> Option<&Item> {
        if self.max_space < 1 { return None; }
        for c in self.contents.values() {
            if c.id() == id { return Some(c); }
            if let Some(item) = c.peek_at(id) {
                return Some(item);
            }
        }
        None
    }

    fn take(&mut self, id: &str) -> Option<Item> {
        if self.max_space < 1 { return None; }
        if let Some(item) = self.contents.remove(id) {
            return Some(item);
        }
        for c in self.contents.values_mut() {
            if let Some(item) = c.take(id) {
                return Some(item);
            }
        }
        None
    }
}
