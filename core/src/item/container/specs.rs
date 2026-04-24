//! Container specs themselves.

use std::{cmp::Ordering, collections::HashMap};

use async_trait::async_trait;
use cosmic_garden_pm::{IdentityMut, ItemizedMut, OwnedMut};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{r#const::{HUGE_ITEM, SMALL_ITEM, TINY_ITEM}, identity::IdentityQuery, item::{Item, Itemized, StorageQueryError, container::{Storage, StorageError, StorageMut, variants::{ContainerVariant, CorpseSpec}}, ownership::Owner}, string::{Describable, DescribableMut, UNNAMED, Uuid}, traits::{Reflector, Tickable}};

/// "Unit" of space and/or weight…
pub type StorageSpace = u16;

/// Max space spec. definer for some common container types.
pub enum MaxSpaceSpec {
    Pouch,
    Backpack,
    Chest,
}

impl From<MaxSpaceSpec> for StorageSpace {
    /// Derive max [StorageSpace] from [MaxSpaceSpec].
    fn from(value: MaxSpaceSpec) -> Self {
        match value {
            MaxSpaceSpec::Pouch => TINY_ITEM * 5,
            MaxSpaceSpec::Backpack => SMALL_ITEM * 10,
            MaxSpaceSpec::Chest => HUGE_ITEM * 6,
        }
    }
}

impl From<StorageSpace> for MaxSpaceSpec {
    /// Derive [MaxSpaceSpec] from an arbitrary [StorageSpace] value.
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
        size: 20,
        desc: "a backpack".into(),
        desc_can_be_modified: true,
        owner: Owner::no_one(),
    };

    pub(super) static ref DEFAULT_POUCH_SPEC: ContainerSpec = ContainerSpec {
        id: "pouch".with_uuid(),
        name: "pouch".into(),
        contents: HashMap::new(),
        max_space: StorageSpace::from(MaxSpaceSpec::Pouch),
        size: 10,
        desc: "a pouch".into(),
        desc_can_be_modified: true,
        owner: Owner::no_one(),
    };

    pub(super) static ref DEFAULT_PLR_INV_SPEC: ContainerSpec = ContainerSpec {
        id: "player-inventory".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 500,
        size: 0,
        desc: "player-inventory".into(),
        desc_can_be_modified: false,
        owner: Owner::no_one(),
    };

    pub(super) static ref DEFAULT_ROOM_SPACE_SPEC: ContainerSpec = ContainerSpec {
        id: "room-space".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 10_000,
        size: 0,
        desc: "room-space".into(),
        desc_can_be_modified: false,
        owner: Owner::no_one(),
    };

    pub(super) static ref DEFAULT_CHEST_SPEC: ContainerSpec = ContainerSpec {
        id: "chest".with_uuid(),
        name: "chest".into(),
        contents: HashMap::new(),
        max_space: StorageSpace::from(MaxSpaceSpec::Chest),
        size: StorageSpace::from(MaxSpaceSpec::Chest),
        desc: "a chest".into(),
        desc_can_be_modified: true,
        owner: Owner::no_one(),
    };
}

/// Container specs dwell here…
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, ItemizedMut, OwnedMut)]
pub struct ContainerSpec {
    pub id: String,
    #[identity(title)]
    pub name: String,
    pub owner: Owner,
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
    /// Make a reflection of self.
    /// 
    /// Reflection is basically (an empty) clone but with different ID.
    fn from(value: &ContainerSpec) -> Self {
        Self {
            id: value.id().re_uuid(),
            name: value.name.clone(),
            contents: HashMap::new(),
            max_space: value.max_space,
            size: value.size,
            desc: value.desc.clone(),
            desc_can_be_modified: value.desc_can_be_modified,
            owner: value.owner.clone(),
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
    /// Calculate how much [StorageSpace] the contents, if any, take.
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
                ContainerVariant::Room(v)  => v.eq(self),
                ContainerVariant::Corpse(CorpseSpec { spec, ..}) => spec.eq(self),
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
                ContainerVariant::Corpse(CorpseSpec { spec, ..}) => self.partial_cmp(spec),
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

    fn can_hold(&self, item: &Item) -> Result<(), StorageQueryError>
    {
        if self < item {
            return Err(StorageQueryError::InvalidHierarchy);
        }
        let ok = item.required_space() as usize
            + self.contents_size() as usize
            <= self.max_space as usize;
        if ok {
            Ok(())
        } else {
            Err(StorageQueryError::NoSpace)
        }
    }

    fn try_insert(&mut self, item: Item) -> Result<(), StorageError>
    {
        if let Err(e) = self.can_hold(&item) {
            return Err(match e {
                StorageQueryError::NoSpace => StorageError::NoSpace(item),
                StorageQueryError::InvalidHierarchy => StorageError::InvalidHierarchy(item),
                StorageQueryError::NotContainer => StorageError::NotContainer(item),
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

#[async_trait]
impl Tickable for ContainerSpec {
    async fn tick(&mut self) -> bool {
        let mut ticked = false;
        for i in self.contents.values_mut() {
            let t = i.tick().await;
            if t { ticked = true; }
        }
        #[cfg(debug_assertions)]{
            if ticked {log::debug!("{} contents ticked.", self.id)}
        }
        ticked
    }
}
