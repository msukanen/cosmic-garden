//! Primordial [Item] which is not yet "anything".

use cosmic_garden_pm::IdentityMut;
use serde::{Deserialize, Serialize};

use crate::{item::{Item, Itemized, container::specs::StorageSpace}, string::{Describable, Uuid}, traits::Reflector};

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
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut)]
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

impl Itemized for PrimordialItem {
    fn size(&self) -> StorageSpace {
        self.size
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
