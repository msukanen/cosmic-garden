use cosmic_garden_pm::{IdentityMut, Itemized, Storage};
use serde::{Deserialize, Serialize};

use crate::item::{Item, container::{Storage, specs::{ContainerSpec, DEFAULT_BACKPACK_SPEC, DEFAULT_PLR_INV_SPEC, DEFAULT_POUCH_SPEC, DEFAULT_ROOM_SPACE_SPEC}}};

pub enum ContainerVariantType {
    Pouch,
    Backpack,
    PlayerInventory,
    Room,
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
}
