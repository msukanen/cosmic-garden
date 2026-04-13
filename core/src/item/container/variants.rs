use std::i32;

use cosmic_garden_pm::{IdentityMut, Storage, OwnedMut};
use serde::{Deserialize, Serialize};

use crate::{item::{Item, Itemized, ItemizedMut, container::{StorageMut, specs::{ContainerSpec, DEFAULT_BACKPACK_SPEC, DEFAULT_CHEST_SPEC, DEFAULT_PLR_INV_SPEC, DEFAULT_POUCH_SPEC, DEFAULT_ROOM_SPACE_SPEC, StorageSpace}}}, string::{Describable, DescribableMut}, traits::{Reflector, Tickable}};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ContainerVariantType {
    Pouch,
    Backpack,
    Chest,
    PlayerInventory,
    Room,
}

impl ContainerVariantType {
    pub fn rank(&self) -> i32 {
        match self {
            Self::Pouch => 0,
            Self::Backpack => 10,
            Self::Chest => 50,
            Self::PlayerInventory => 100,
            Self::Room => i32::MAX,
        }
    }
}

impl From<&ContainerVariant> for ContainerVariantType {
    fn from(value: &ContainerVariant) -> Self {
        match value {
            ContainerVariant::Backpack(_) => Self::Backpack,
            ContainerVariant::PlayerInventory(_) => Self::PlayerInventory,
            ContainerVariant::Pouch(_) => Self::Pouch,
            ContainerVariant::Room(_) => Self::Room,
            ContainerVariant::Chest(_) => Self::Chest,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, Storage, OwnedMut)]
pub enum ContainerVariant {
    Pouch(ContainerSpec),
    Backpack(ContainerSpec),
    Chest(ContainerSpec),
    PlayerInventory(ContainerSpec),
    Room(ContainerSpec),
}

impl Describable for ContainerVariant {
    fn desc<'a>(&'a self) -> &'a str {
        match self {
            Self::Backpack(v) |
            Self::PlayerInventory(v) |
            Self::Pouch(v) |
            Self::Chest(v) |
            Self::Room(v) => v.desc()
        }
    }
}

impl DescribableMut for ContainerVariant {
    fn set_desc(&mut self, text: &str) -> bool {
        match self {
            Self::Backpack(v) |
            Self::PlayerInventory(v) |
            Self::Pouch(v) |
            Self::Chest(v) |
            Self::Room(v) => v.set_desc(text)
        }
    }
}

impl ContainerVariant {
    /// Get new [ContainerVariant] as [Item].
    pub fn new(variant: ContainerVariantType) -> Item {
        Item::Container(Self::raw(variant))
    }

    /// Get new pure [ContainerVariant].
    pub fn raw(variant: ContainerVariantType) -> Self {
        match variant {
            ContainerVariantType::Backpack => Self::Backpack(ContainerSpec::from(&*DEFAULT_BACKPACK_SPEC)),
            ContainerVariantType::PlayerInventory => Self::PlayerInventory(ContainerSpec::from(&*DEFAULT_PLR_INV_SPEC)),
            ContainerVariantType::Pouch => Self::Pouch(ContainerSpec::from(&*DEFAULT_POUCH_SPEC)),
            ContainerVariantType::Room => Self::Room(ContainerSpec::from(&*DEFAULT_ROOM_SPACE_SPEC)),
            ContainerVariantType::Chest => Self::Chest(ContainerSpec::from(&*DEFAULT_CHEST_SPEC)),
        }
    }

    pub fn rank(&self) -> i32 {
        self.variant_type().rank()
    }

    pub fn variant_type(&self) -> ContainerVariantType {
        ContainerVariantType::from(self)
    }
}

impl Reflector for ContainerVariant {
    fn reflect(&self) -> Self {
        match self {
            Self::Backpack(b) => Self::Backpack(b.reflect()),
            Self::PlayerInventory(p) => Self::PlayerInventory(p.reflect()),
            Self::Pouch(p) => Self::Pouch(p.reflect()),
            Self::Room(r) => Self::Room(r.reflect()),
            Self::Chest(c) => Self::Chest(c.reflect()),
        }
    }

    fn deep_reflect(&self) -> Self {
        match self {
            Self::Backpack(b) => Self::Backpack(b.deep_reflect()),
            Self::PlayerInventory(p) => Self::PlayerInventory(p.deep_reflect()),
            Self::Pouch(p) => Self::Pouch(p.deep_reflect()),
            Self::Room(r) => Self::Room(r.deep_reflect()),
            Self::Chest(c) => Self::Chest(c.deep_reflect()),
        }
    }
}

impl Itemized for ContainerVariant {
    fn size(&self) -> StorageSpace {
        match self {
            Self::Backpack(v) |
            Self::Pouch(v) |
            Self::PlayerInventory(v) |
            Self::Chest(v) |
            Self::Room(v) => v.size()
        }
    }
}

impl ItemizedMut for ContainerVariant {
    fn set_size(&mut self, sz: StorageSpace) -> bool {
        match self {
            Self::PlayerInventory(_) |
            Self::Room(_) => false,
            
            Self::Backpack(v) |
            Self::Chest(v) |
            Self::Pouch(v) => v.set_size(sz)
        }
    }
}

impl StorageMut for ContainerVariant {
    fn set_max_space(&mut self, sz: StorageSpace) -> bool {
        match self {
            Self::PlayerInventory(_) |
            Self::Room(_) => false,

            Self::Backpack(v) |
            Self::Chest(v) |
            Self::Pouch(v) => v.set_max_space(sz),
        }
    }
}

impl<'a> IntoIterator for &'a ContainerVariant {
    type Item = (&'a String, &'a Item);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ContainerVariant::Backpack(v) |
            ContainerVariant::Chest(v) |
            ContainerVariant::Pouch(v) |
            ContainerVariant::Room(v) |
            ContainerVariant::PlayerInventory(v) => v.into_iter()
        }
    }
}

impl Tickable for ContainerVariant {
    fn tick(&mut self) -> bool {
        match self {
            Self::Backpack(c) => c.tick(),
            Self::Chest(c) => c.tick(),
            Self::PlayerInventory(c) => c.tick(),
            Self::Pouch(c) => c.tick(),
            Self::Room(c) => c.tick(),
        }
    }
}
