//! Primordial [Item] which is not yet "anything".

use std::{collections::HashMap, fmt::Display};

use cosmic_garden_pm::{IdentityMut, ItemizedMut};
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{Item, Itemized, container::{Storage, StorageMut, specs::{ContainerSpec, MaxSpaceSpec, StorageSpace}, variants::ContainerVariant}}, string::{Describable, Uuid}, traits::Reflector};

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

impl Display for PotentialItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Consumable => "consumable",
            Self::Container => "container",
            Self::Key => "key",
            Self::Other_ => "other/primordial",
            Self::Tool => "tool",
            Self::Weapon => "weapon"
        })
    }
}

#[derive(Debug, Clone)]
pub enum PotentialItemTypeError {
    ListAll,
    Ambiguous,
}

impl Display for PotentialItemTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Ambiguous => "<c yellow>cons</c>umable, <c yellow>cont</c>ainer",
            Self::ListAll => "<c yellow>cons</c>umable, <c yellow>cont</c>ainer, <c yellow>k</c>ey, <c yellow>t</c>ool, <c yellow>w</c>eapon"
        })
    }
}

impl PotentialItemType {
    pub fn from(value: &str) -> Result<Self, PotentialItemTypeError> {
        if value.len() < 4 && value.starts_with("con") { return Err(PotentialItemTypeError::Ambiguous);}
        if value.starts_with("cont") { return Ok(Self::Container); }
        if value.starts_with("cons") { return Ok(Self::Consumable);}
        match value.chars().nth(0) {
            None => Err(PotentialItemTypeError::ListAll),
            Some(c) => Ok(match c {
                'w' => Self::Weapon,
                't' => Self::Tool,
                'k' => Self::Weapon,
                'o' => Self::Other_,// TODO might need to disable this later
                _ => return Err(PotentialItemTypeError::ListAll)
            })
        }
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

    pub fn set_potential(&mut self, pot: PotentialItemType) -> bool {
        self.potential = pot;
        true
    }

    pub fn atomize(item: &Item) -> Item {
        match item {
            Item::Primordial(_) => item.clone(),
            _ => Item::Primordial(
                PrimordialItem {
                    id: item.id().to_string(),
                    title: item.title().to_string(),
                    size: item.size(),
                    desc: item.desc().to_string(),
                    max_space: item.max_space(),
                    potential: PotentialItemType::Other_,
                }
            )
        }
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
