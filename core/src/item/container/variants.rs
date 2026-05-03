use async_trait::async_trait;
use cosmic_garden_pm::{IdentityMut, Storage, OwnedMut};
use serde::{Deserialize, Serialize};

pub mod corpse; pub use corpse::{CorpseSpec, bulk_transfer};

use crate::{
    r#const::SIZE_BALANCE,
    identity::uniq::Uuid,
    item::{
        Item, Itemized, ItemizedMut,
        container::{
            ContainerSpec, DEFAULT_BACKPACK_SPEC, DEFAULT_CHEST_SPEC, DEFAULT_PLR_INV_SPEC, DEFAULT_POUCH_SPEC, DEFAULT_ROOM_SPACE_SPEC, StorageMut, StorageSpace
        },
    },
    string::{Describable, DescribableMut},
    traits::{Reflector, Tickable},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ContainerVariantType {
    Pouch,
    Backpack,
    Chest,
    PlayerInventory,
    Room,
    Corpse,
}

impl ContainerVariantType {
    pub fn rank(&self) -> StorageSpace {
        match self {
            Self::Pouch => SIZE_BALANCE * 2,
            Self::Backpack => SIZE_BALANCE * 30,
            Self::Chest => SIZE_BALANCE * 60,
            Self::Corpse => SIZE_BALANCE * 70,
            Self::PlayerInventory => SIZE_BALANCE * 75,
            Self::Room => StorageSpace::MAX,
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
            ContainerVariant::Corpse{..} => Self::Corpse,
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
    Corpse(CorpseSpec),
}


impl Describable for ContainerVariant {
    fn desc<'a>(&'a self) -> &'a str {
        match self {
            Self::Backpack(spec)        |
            Self::PlayerInventory(spec) |
            Self::Pouch(spec)           |
            Self::Chest(spec)           |
            Self::Room(spec) => spec.desc(),
            Self::Corpse(c)     => c.spec.desc(),
        }
    }
}

impl DescribableMut for ContainerVariant {
    fn set_desc(&mut self, text: &str) -> bool {
        match self {
            Self::PlayerInventory(spec) |
            Self::Backpack(spec)        |
            Self::Pouch(spec) |
            Self::Chest(spec) |
            Self::Room(spec)  => spec.set_desc(text),
            Self::Corpse(c) => c.spec.set_desc(text),
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
            ContainerVariantType::Corpse => {
                ContainerVariant::Corpse(
                    CorpseSpec { spec:
                        ContainerSpec {
                            id: "corpse-inventory".re_uuid(),
                            name: "corpse-inventory".to_string(),
                            ..ContainerSpec::from(&*DEFAULT_PLR_INV_SPEC)
                        },
                        possessed_by: None
                    }
                )
            }
        }
    }

    pub fn rank(&self) -> StorageSpace {
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
            Self::Corpse { .. } => self.deep_reflect(),
        }
    }

    fn deep_reflect(&self) -> Self {
        match self {
            Self::Backpack(b) => Self::Backpack(b.deep_reflect()),
            Self::PlayerInventory(p) => Self::PlayerInventory(p.deep_reflect()),
            Self::Pouch(p) => Self::Pouch(p.deep_reflect()),
            Self::Room(r) => Self::Room(r.deep_reflect()),
            Self::Chest(c) => Self::Chest(c.deep_reflect()),
            Self::Corpse(CorpseSpec { spec, possessed_by })
                => Self::Corpse(CorpseSpec { spec: spec.deep_reflect(), possessed_by: possessed_by.clone() })
        }
    }
}

impl Itemized for ContainerVariant {
    fn size(&self) -> StorageSpace {
        match self {
            Self::PlayerInventory(spec) |
            Self::Backpack(spec)        |
            Self::Pouch(spec) |
            Self::Chest(spec) |
            Self::Room(spec)  => spec.size(),
            Self::Corpse(c) => c.spec.size()
        }
    }
}

impl ItemizedMut for ContainerVariant {
    fn set_size(&mut self, sz: StorageSpace) -> bool {
        match self {
            Self::PlayerInventory(_) |
            Self::Corpse { .. }      |
            Self::Room(_)           => false,
            
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
            Self::Corpse { .. }      |
            Self::Room(_)           => false,

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
            ContainerVariant::PlayerInventory(spec)|
            ContainerVariant::Backpack(spec) |
            ContainerVariant::Chest(spec)    |
            ContainerVariant::Pouch(spec)    |
            ContainerVariant::Room(spec)    => spec.into_iter(),
            ContainerVariant::Corpse(c) => c.spec.into_iter(),
        }
    }
}

#[async_trait]
impl Tickable for ContainerVariant {
    async fn tick(&mut self) -> bool {
        match self {
            Self::PlayerInventory(spec)|
            Self::Backpack(spec) |
            Self::Chest(spec)    |
            Self::Pouch(spec)    |
            Self::Room(spec)    => spec.tick().await,
            Self::Corpse(CorpseSpec { spec, possessed_by })
                => {
                    let st = spec.tick().await;
                    let pt = if let Some(poss) = possessed_by {
                        let mut lock = poss.write().await;
                        lock.tick().await
                    } else {
                        false
                    };
                    st || pt
                }
        }
    }
}
