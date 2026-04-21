//! Primordial [Item] which is not yet "anything".

use std::{collections::HashMap, fmt::Display};

use cosmic_garden_pm::{DescribableMut, IdentityMut, ItemizedMut, OwnedMut};
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{Item, Itemized, StorageError, StorageQueryError, consumable::{ConsumableMatter, EffectType}, container::{Storage, StorageMut, specs::{ContainerSpec, MaxSpaceSpec, StorageSpace}, variants::ContainerVariant}, matter::MatterState, ownership::Owner, weapon::{WeaponSize, WeaponSpec}}, mob::StatValue, string::{Describable, Uuid}, traits::{Reflector, Tickable}};

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

/// Entirely "primordial soup" for creating anything and everything.
/// 
/// What anything becomes, depends on what the `potential` is set upon 'weave'.
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, ItemizedMut, DescribableMut, OwnedMut)]
pub struct PrimordialItem {
    pub id: String,
    pub title: String,
    pub desc: String,

    pub owner: Owner,

    pub potential: PotentialItemType,

    pub size: StorageSpace,
    pub max_space: StorageSpace,

    pub uses: Option<usize>,
    
    // Item::Consumable -specific:
    pub nutrition: Option<EffectType>,
    pub affect_ticks: Option<usize>,
    pub rots_in_ticks: Option<usize>,
    pub matter_state: Option<MatterState>,
    // Item::Weapon -specific:
    pub weapon_size: Option<WeaponSize>,
    pub base_dmg: Option<StatValue>,
}

impl Default for PrimordialItem {
    fn default() -> Self {
        Self {
            id: "Primordial".with_uuid(),
            title: "Primordial Soup".into(),
            owner: Owner::no_one(),
            size: 0,
            desc: "Something indescribable…".into(),
            max_space: 0,
            potential: PotentialItemType::default(),
            uses: None,
            nutrition: None,
            affect_ticks: None,
            rots_in_ticks: None,
            matter_state: None,
            weapon_size: None,
            base_dmg: None,
        }
    }
}

impl PrimordialItem {
    pub fn new(id: &str) -> Item {
        Item::Primordial(
            Self { id: id.re_uuid(), ..Self::default() }
        )
    }

    /// Set item potential.
    pub fn set_potential(&mut self, pot: PotentialItemType) -> bool {
        self.potential = pot;
        true
    }

    pub fn potential(&self) -> PotentialItemType { self.potential.clone() }

    /// Atomize any item into primal goo...
    pub fn atomize(item: &Item) -> Item {
        match item {
            Item::Primordial(_) => item.clone(),
            Item::Consumable(item) => Item::Primordial(PrimordialItem::from(item)),
            Item::Weapon(item) => Item::Primordial(PrimordialItem::from(item)),
            _ => todo!("More atoms!")
        }
    }
}

impl From<&ConsumableMatter> for PrimordialItem {
    /// Convert [ConsumableMatter] into [PrimordialItem].
    fn from(matter: &ConsumableMatter) -> Self { Self {
        id: matter.id().into(),
        title: matter.title().into(),
        size: matter.size(),
        desc: matter.desc().into(),
        max_space: 0,
        potential: PotentialItemType::Other_,
        uses: matter.uses,
        nutrition: matter.nutrition.clone().into(),
        affect_ticks: matter.affect_ticks.clone(),
        rots_in_ticks: matter.rots_in_ticks.clone(),
        matter_state: matter.matter_state.into(),
        owner: matter.owner.clone(),
        weapon_size: None,
        base_dmg: None,
    }}
}
impl From<ConsumableMatter> for PrimordialItem {
    /// Convert [ConsumableMatter] into [PrimordialItem].
    fn from(value: ConsumableMatter) -> Self { Self::from(&value) }
}

impl From<&WeaponSpec> for PrimordialItem {
    /// Convert [WeaponSpec] into [PrimordialItem].
    fn from(value: &WeaponSpec) -> Self { Self {
        id: value.id().into(),
        title: value.title().into(),
        desc: value.desc().into(),
        owner: value.owner.clone().into(),
        potential: PotentialItemType::Other_,
        size: value.size(),
        max_space: 0,
        uses: None,
        nutrition: None,
        affect_ticks: None,
        rots_in_ticks: None,
        matter_state: None,
        weapon_size: value.weapon_size.into(),
        base_dmg: value.base_dmg.into()
    }}
}
impl From<WeaponSpec> for PrimordialItem {
    /// Convert [WeaponSpec] into [PrimordialItem].
    fn from(value: WeaponSpec) -> Self { Self::from(&value)}
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

/// [PrimordialItem] fulfils the [Storage] contract, but as it isn't a true container,
/// most of these impls are just stubs, or report "no can do".
impl Storage for PrimordialItem {
    /// Always `Err`; [PrimordialItem] is never a true container.
    #[inline]
    fn can_hold(&self, _item: &Item) -> Result<(), StorageQueryError> {
        Err(StorageQueryError::NotContainer)
    }
    /// Always `false`; [PrimordialItem] is never a true container.
    #[inline]
    fn contains(&self, _: &str) -> bool { false }
    /// Always `None`; [PrimordialItem] is never a true container.
    #[inline]
    fn eject_all(&mut self) -> Option<Vec<Item>> { None }
    /// Always `None`; [PrimordialItem] is never a true container.
    #[inline]
    fn find_id_by_name(&self, _: &str) -> Option<String> { None }
    fn max_space(&self) -> StorageSpace { self.max_space }
    /// Always `None`; [PrimordialItem] is never a true container.
    #[inline]
    fn peek_at(&self, _: &str) -> Option<&Item> { None }
    fn required_space(&self) -> StorageSpace { self.size }
    /// Always `0`; [PrimordialItem] is never a true container.
    #[inline]
    fn space(&self) -> StorageSpace { 0 }
    /// Always `None`; [PrimordialItem] is never a true container.
    #[inline]
    fn take(&mut self, _: &str) -> Option<Item> { None }
    /// Always `None`; [PrimordialItem] is never a true container.
    #[inline]
    fn take_by_name(&mut self, _: &str) -> Option<Item> { None }
    /// Always `Err(item)`; [PrimordialItem] is never a true container.
    #[inline]
    fn try_insert(&mut self, item: Item) -> Result<(), StorageError> { Err(StorageError::NotContainer(item)) }
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
                // staying as-is?
                PotentialItemType::Other_ => Item::Primordial(self),
                PotentialItemType::Container => unimplemented!("Already determined by max_space"),

                PotentialItemType::Consumable =>
                    Item::Consumable(ConsumableMatter {
                        id: self.id.clone(),
                        title: self.title,
                        size: self.size,
                        nutrition: self.nutrition.unwrap_or(EffectType::NotEdible),
                        desc: self.desc,
                        uses: self.uses,
                        affect_ticks: self.affect_ticks,
                        rots_in_ticks: self.rots_in_ticks,
                        matter_state: self.matter_state.unwrap_or_else(|| {
                            log::warn!("Builder: consumable '{}' lacks proper matter_state! Going \"safe\" with 'solid' assumption.", self.id);
                            MatterState::Solid
                        }),
                        owner: self.owner.clone(),
                    }),
                
                PotentialItemType::Weapon =>
                    Item::Weapon(WeaponSpec {
                        id: self.id.clone(),
                        name: self.title,
                        desc: self.desc,
                        owner: Owner::no_one(),
                        size: self.size,
                        weapon_size: self.weapon_size.unwrap_or_else(|| {
                            log::warn!("Builder: weapon '{}' lacks weapon_size. Going \"safe\" with 'medium' assumption.", self.id);
                            WeaponSize::Medium
                        }),
                        base_dmg: self.base_dmg.unwrap_or_else(|| {
                            log::warn!("Builder: weapon '{}' lacks base_dmg. Going \"safe\" with 0.0.", self.id);
                            0.0
                        }),
                    }),
                
                PotentialItemType::Key |
                PotentialItemType::Tool  => todo!("TODO: more item modes!"),
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
