//! Primordial [Item] which is not yet "anything".

use std::{collections::HashMap, fmt::Display};

use cosmic_garden_pm::{DescribableMut, IdentityMut, ItemizedMut, OwnedMut};
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{Item, Itemized, consumable::{ConsumableMatter, NutritionType}, container::{StorageMut, specs::{ContainerSpec, MaxSpaceSpec, StorageSpace}, variants::ContainerVariant}, matter::MatterState, ownership::{ItemSource, Owned, Owner}}, string::{Describable, Uuid}, traits::{Reflector, Tickable}};

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
                'k' => Self::Key,
                'o' => Self::Other_,// TODO might need to disable this later
                _ => return Err(PotentialItemTypeError::ListAll)
            })
        }
    }
}

/// Entirely "primordial soup" for creating other items from.
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, ItemizedMut, DescribableMut, OwnedMut)]
pub struct PrimordialItem {
    pub id: String,
    pub title: String,
    pub owner: Owner,
    pub size: StorageSpace,
    pub desc: String,
    pub max_space: StorageSpace,
    pub potential: PotentialItemType,
    pub uses: Option<usize>,
    pub nutrition: Option<NutritionType>,
    pub affect_ticks: Option<usize>,
    pub rots_in_ticks: Option<usize>,
    pub matter_state: Option<MatterState>,
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
            uses: None,
            nutrition: None,
            affect_ticks: None,
            rots_in_ticks: None,
            matter_state: None,
            owner: Owner::no_one(),
        })
    }

    pub fn set_potential(&mut self, pot: PotentialItemType) -> bool {
        self.potential = pot;
        true
    }

    pub fn potential(&self) -> PotentialItemType { self.potential.clone() }

    /// Atomize any item into primal goo...
    pub fn atomize(item: &Item) -> Item {
        match item {
            Item::Primordial(_) => item.clone(),
            Item::Consumable(item) => Item::Primordial(PrimordialItem {
                    id: item.id().to_string(),
                    title: item.title().to_string(),
                    size: item.size(),
                    desc: item.desc().to_string(),
                    max_space: 0,
                    potential: PotentialItemType::Other_,
                    uses: item.uses,
                    nutrition: item.nutrition.clone().into(),
                    affect_ticks: item.affect_ticks.clone(),
                    rots_in_ticks: item.affect_ticks.clone(),
                    matter_state: item.matter_state.into(),
                    owner: item.owner.clone(),
            }),
            _ => todo!("More atoms!")
        }
    }
}

impl Reflector for PrimordialItem {
    fn reflect(&self) -> Self {
        Self { id: self.id.re_uuid(), ..self.clone() }
    }
    fn deep_reflect(&self) -> Self {
        self.reflect()
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
                desc_can_be_modified: true,
                owner: self.owner.clone(),
            };
            Item::Container(match vessel_model {
                MaxSpaceSpec::Backpack => ContainerVariant::Backpack(spec),
                MaxSpaceSpec::Chest => ContainerVariant::Chest(spec),
                MaxSpaceSpec::Pouch => ContainerVariant::Pouch(spec)
            })
        } else {
            match self.potential {
                PotentialItemType::Container => unimplemented!("Already determined by max_space"),
                PotentialItemType::Consumable => {
                    Item::Consumable(ConsumableMatter {
                        id: self.id.clone(),
                        title: self.title,
                        size: self.size,
                        nutrition: self.nutrition.unwrap_or(NutritionType::NotEdible),
                        desc: self.desc,
                        uses: self.uses,
                        affect_ticks: self.affect_ticks,
                        rots_in_ticks: self.rots_in_ticks,
                        matter_state: self.matter_state.unwrap_or_else(|| {
                            log::warn!("Builder: consumable '{}' lacks proper matter_state! Going \"safe\" with 'solid' assumption.", self.id);
                            MatterState::Solid
                        }),
                        owner: self.owner.clone(),
                    })
                }
                // staying as-is?
                PotentialItemType::Other_ => Item::Primordial(self),
                PotentialItemType::Weapon |
                PotentialItemType::Key |
                PotentialItemType::Tool  => todo!("TODO: more item modes!")
            }
        }
    }
}

impl Tickable for PrimordialItem {
    fn tick(&mut self) -> bool {
        #[cfg(all(debug_assertions, feature = "stresstest"))]{
            log::debug!("Primordial '{}' ticked.", self.id);
            return true;
        }
        false
    }
}
