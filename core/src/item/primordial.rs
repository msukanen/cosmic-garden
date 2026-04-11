//! Primordial [Item] which is not yet "anything".

use std::collections::HashMap;

use cosmic_garden_pm::{IdentityMut, ItemizedMut};
use serde::{Deserialize, Serialize};

use crate::{item::{Item, container::{StorageMut, specs::{ContainerSpec, MaxSpaceSpec, StorageSpace}, variants::ContainerVariant}}, string::{Describable, Uuid}, traits::Reflector};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PotentialItemType {
    Container,
    Weapon,
    Tool,
    Key,
    Consumable,

    Other_,
}

impl Default for PotentialItemType {
    fn default() -> Self {
        Self::Other_
    }
}

/// Entirely "primordial soup" for creating other items from.
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, ItemizedMut)]
pub struct PrimordialItem {
    pub id: String,
    pub title: String,
    pub size: StorageSpace,
    pub desc: String,
    pub max_space: StorageSpace,
    pub potential: PotentialItemType,
}

impl PrimordialItem {
    pub fn new(id: &str) -> Item {
        Item::Primordial(Self {
            id: id.re_uuid(),
            title: "Primordial Soup".into(),
            size: 0,
            desc: "Something indescribable…".into(),
            max_space: 0,
            potential: PotentialItemType::default(),
        })
    }
}

impl Reflector for PrimordialItem {
    fn reflect(&self) -> Self {
        self.clone()
    }
}

impl Describable for PrimordialItem {
    fn desc<'a>(&'a self) -> &'a str {
        &self.desc
    }

    fn set_desc(&mut self, text: &str) -> bool {
        self.desc = text.to_string();
        true
    }
}

impl StorageMut for PrimordialItem {
    fn set_max_space(&mut self, sz: StorageSpace) -> bool {
        self.max_space = sz;
        true
    }
}

pub trait Metamorphize {
    fn metamorph(self) -> Item;
}

impl Metamorphize for PrimordialItem {
    fn metamorph(self) -> Item {
        // figure out the Final Form based on specs…

        if self.max_space > 0 {
            let vessel_model = MaxSpaceSpec::from(self.max_space);
            let spec = ContainerSpec {
                id: self.id,
                name: self.title,
                contents: HashMap::new(),
                max_space: self.max_space,
                size: self.size,
                desc: self.desc,
                desc_can_be_modified: true
            };
            Item::Container(match vessel_model {
                MaxSpaceSpec::Backpack => ContainerVariant::Backpack(spec),
                MaxSpaceSpec::Chest => ContainerVariant::Chest(spec),
                MaxSpaceSpec::Pouch => ContainerVariant::Pouch(spec)
            })
        } else {
            match self.potential {
                PotentialItemType::Container => unimplemented!("Already determined by max_space"),
                // staying as-is?
                PotentialItemType::Other_ => Item::Primordial(self),
                PotentialItemType::Weapon |
                PotentialItemType::Key |
                PotentialItemType::Tool |
                PotentialItemType::Consumable => todo!("TODO: more item modes!")
            }
        }
    }
}
