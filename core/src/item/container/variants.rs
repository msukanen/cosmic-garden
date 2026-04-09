use std::i32;

use cosmic_garden_pm::{IdentityMut, Itemized, Storage};
use serde::{Deserialize, Serialize};

use crate::{item::{Item, container::{Storage, specs::{ContainerSpec, DEFAULT_BACKPACK_SPEC, DEFAULT_PLR_INV_SPEC, DEFAULT_POUCH_SPEC, DEFAULT_ROOM_SPACE_SPEC}}}, traits::Reflector};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ContainerVariantType {
    Pouch,
    Backpack,
    PlayerInventory,
    Room,
}

impl ContainerVariantType {
    pub fn rank(&self) -> i32 {
        match self {
            Self::Pouch => 0,
            Self::Backpack => 10,
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
        }
    }
}

#[derive(Debug, Deserialize, Serialize, IdentityMut, Storage, Itemized)]
pub enum ContainerVariant {
    Pouch(ContainerSpec),
    Backpack(ContainerSpec),
    PlayerInventory(ContainerSpec),
    Room(ContainerSpec),
}

impl ContainerVariant {
    pub fn new(variant: ContainerVariantType) -> Item {
        Item::Container(match variant {
            ContainerVariantType::Backpack => Self::Backpack(ContainerSpec::from(&*DEFAULT_BACKPACK_SPEC)),
            ContainerVariantType::PlayerInventory => Self::PlayerInventory(ContainerSpec::from(&*DEFAULT_PLR_INV_SPEC)),
            ContainerVariantType::Pouch => Self::Pouch(ContainerSpec::from(&*DEFAULT_POUCH_SPEC)),
            ContainerVariantType::Room => Self::Room(ContainerSpec::from(&*DEFAULT_ROOM_SPACE_SPEC)),
        })
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
        }
    }
}
