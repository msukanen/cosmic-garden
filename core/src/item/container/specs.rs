//! Container specs themselves.

use std::{cmp::Ordering, collections::HashMap};

use cosmic_garden_pm::{IdentityMut, ItemizedMut};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{Item, Itemized, container::{Storage, StorageError, StorageMut, variants::ContainerVariant}}, string::{Describable, DescribableMut, UNNAMED, Uuid}, traits::Reflector};

pub type StorageSpace = u16;

pub enum MaxSpaceSpec {
    Pouch,
    Backpack,
    Chest,
}

impl From<MaxSpaceSpec> for StorageSpace {
    fn from(value: MaxSpaceSpec) -> Self {
        match value {
            MaxSpaceSpec::Pouch => 10,
            MaxSpaceSpec::Backpack => 30,
            MaxSpaceSpec::Chest => 80,
        }
    }
}

impl From<StorageSpace> for MaxSpaceSpec {
    fn from(value: StorageSpace) -> Self {
        match value {
            _ if value > StorageSpace::from(MaxSpaceSpec::Backpack) => MaxSpaceSpec::Chest,
            _ if value > StorageSpace::from(MaxSpaceSpec::Pouch) => MaxSpaceSpec::Backpack,
            _ => MaxSpaceSpec::Pouch
        }
    }
}

lazy_static! {
    pub(super) static ref DEFAULT_BACKPACK_SPEC: ContainerSpec = ContainerSpec {
        id: "backpack".with_uuid(),
        name: "backpack".into(),
        contents: HashMap::new(),
        max_space: StorageSpace::from(MaxSpaceSpec::Backpack),
        size: 2,
        desc: "a backpack".into(),
        desc_can_be_modified: true,
    };

    pub(super) static ref DEFAULT_POUCH_SPEC: ContainerSpec = ContainerSpec {
        id: "pouch".with_uuid(),
        name: "pouch".into(),
        contents: HashMap::new(),
        max_space: StorageSpace::from(MaxSpaceSpec::Pouch),
        size: 1,
        desc: "a pouch".into(),
        desc_can_be_modified: true,
    };

    pub(super) static ref DEFAULT_PLR_INV_SPEC: ContainerSpec = ContainerSpec {
        id: "player-inventory".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 50,
        size: 0,
        desc: "player-inventory".into(),
        desc_can_be_modified: false,
    };

    pub(super) static ref DEFAULT_ROOM_SPACE_SPEC: ContainerSpec = ContainerSpec {
        id: "room-space".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 1_000,
        size: 0,
        desc: "room-space".into(),
        desc_can_be_modified: false,
    };

    pub(super) static ref DEFAULT_CHEST_SPEC: ContainerSpec = ContainerSpec {
        id: "chest".with_uuid(),
        name: "chest".into(),
        contents: HashMap::new(),
        max_space: StorageSpace::from(MaxSpaceSpec::Chest),
        size: StorageSpace::from(MaxSpaceSpec::Chest),
        desc: "a chest".into(),
        desc_can_be_modified: true,
    };
}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, ItemizedMut)]
pub struct ContainerSpec {
    pub id: String,
    #[identity(title)]
    pub name: String,
    pub contents: HashMap<String, Item>,
    pub max_space: StorageSpace,
    pub size: StorageSpace,
    pub desc: String,
    pub desc_can_be_modified: bool,
}

impl Describable for ContainerSpec {
    fn desc<'a>(&'a self) -> &'a str {
        &self.desc
    }
}

impl DescribableMut for ContainerSpec {
    fn set_desc(&mut self, text: &str) -> bool {
        if self.desc_can_be_modified {
            self.desc = text.to_string();
        }
        self.desc_can_be_modified
    }
}

impl From<&ContainerSpec> for ContainerSpec {
    fn from(value: &ContainerSpec) -> Self {
        Self {
            id: value.id().re_uuid(),
            name: value.name.clone(),
            contents: HashMap::new(),
            max_space: value.max_space,
            size: value.size,
            desc: value.desc.clone(),
            desc_can_be_modified: value.desc_can_be_modified,
        }
    }
}

impl Reflector for ContainerSpec {
    fn reflect(&self) -> Self {
        Self::from(self)
    }
    fn deep_reflect(&self) -> Self {
        let mut r = Self::from(self);
        for (_,x) in &self.contents {
            let refl = x.reflect();
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
                ContainerVariant::Chest(v) |
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
                ContainerVariant::Chest(v) |
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

    fn take_by_name(&mut self, name: &str) -> Option<Item> {
        if let Some(id) = self.find_id_by_name(name) {
            self.contents.remove(&id)
        } else {
            None
        }
    }

    fn find_id_by_name(&self, name: &str) -> Option<String> {
        let name_lc = name.to_lowercase();
        self.into_iter()
            .find(|(id, item)| {
                // match by UUID, title, or id-prefix
                *id == name ||
                item.title().to_lowercase().contains(&name_lc) ||
                id.starts_with(name)
            })
            .map(|(id, _)| id.clone())
    }

    fn eject_all(&mut self) -> Option<Vec<Item>> {
        if self.contents.is_empty() {
            None
        } else {
            Some(self.contents.drain().map(|(_,v)| v).collect::<Vec<_>>())
        }
    }
}

impl StorageMut for ContainerSpec {
    fn set_max_space(&mut self, sz: StorageSpace) -> bool {
        if self.contents_size() > sz {
            return false;
        }
        self.max_space = sz;
        true
    }
}

impl<'a> IntoIterator for &'a ContainerSpec {
    type Item = (&'a String, &'a Item);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.contents.iter())
    }
}
