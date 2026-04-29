//! Items dwell here…

use async_trait::async_trait;
use cosmic_garden_pm::{IdentityMut, Itemized, OwnedMut};
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{consumable::ConsumableMatter, container::{Storage, StorageMut, specs::StorageSpace, variants::ContainerVariant}, ownership::Owner, primordial::{Metamorphize, PrimordialItem}, weapon::WeaponSpec}, string::{Describable, DescribableMut}, util::uuid::Uuid, traits::{Reflector, Tickable}};

pub mod blueprint; pub use blueprint::BlueprintLibrary;
pub mod consumable;
pub mod container; pub use container::{StorageError, StorageQueryError};
pub mod key;
pub mod matter;
pub mod ownership;
pub mod primordial;
pub mod tool;
pub mod weapon;

pub trait Itemized {
    fn size(&self) -> StorageSpace;
}

pub trait ItemizedMut {
    fn set_size(&mut self, sz: StorageSpace) -> bool;
}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, OwnedMut)]
pub struct TemporaryStructToAppeaseAnalyzerDuringWIP {
    pub(crate) id:String,
    pub(crate) title:String,
    pub(crate) owner:Owner,
}

impl Itemized for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn size(&self) -> StorageSpace {
        1
    }
}
impl ItemizedMut for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn set_size(&mut self, _: StorageSpace) -> bool {
        false
    }
}
impl Reflector for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn reflect(&self) -> Self {
        Self { id: self.id().re_uuid(), title: self.title.clone(), owner:Owner::blueprint() }
    }
    fn deep_reflect(&self) -> Self {
        self.reflect()
    }
}
impl Describable for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn desc<'a>(&'a self) -> &'a str {
        "nothing to see here"
    }
}

impl DescribableMut for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn set_desc(&mut self, _: &str) -> bool {
        false
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, OwnedMut)]
/// Root [Item] types.
pub enum Item {
    Container(ContainerVariant),
    Weapon(WeaponSpec),
    Tool(TemporaryStructToAppeaseAnalyzerDuringWIP),
    Key(TemporaryStructToAppeaseAnalyzerDuringWIP),
    Consumable(ConsumableMatter),
    Primordial(PrimordialItem),
    Corpse(ContainerVariant),
}

impl PartialEq<str> for Item {
    fn eq(&self, other: &str) -> bool {
        let what = other.to_lowercase();
        self.id().starts_with(&what) || self.title().to_lowercase().starts_with(&what)
    }
}

impl Item {
    pub fn devolve(&mut self) {
        match self {
            // Primordial and Corpses don't devolve…
            Self::Primordial(_) => (),
            Self::Corpse(_) => (),
            // …while everything else does.
            Self::Consumable(_) => *self = PrimordialItem::atomize(self),
            Self::Container(_) => *self = PrimordialItem::atomize(self),
            Self::Key(_) => *self = PrimordialItem::atomize(self),
            Self::Tool(_) => *self = PrimordialItem::atomize(self),
            Self::Weapon(_) => *self = PrimordialItem::atomize(self),
        }
    }
}

impl Reflector for Item {
    fn reflect(&self) -> Self {
        match self {
            Self::Container(c) => Self::Container(c.reflect()),
            Self::Corpse(c) => Self::Corpse(c.reflect()),
            Self::Key(k) => Self::Key(k.reflect()),
            Self::Tool(t) => Self::Tool(t.reflect()),
            Self::Weapon(w) => Self::Weapon(w.reflect()),
            Self::Primordial(p) => Self::Primordial(p.reflect()),
            Self::Consumable(c) => Self::Consumable(c.reflect()),
        }
    }
    fn deep_reflect(&self) -> Self {
        match self {
            Self::Container(c) => Self::Container(c.deep_reflect()),
            Self::Corpse(c) => Self::Corpse(c.deep_reflect()),
            Self::Key(k) => Self::Key(k.deep_reflect()),
            Self::Tool(t) => Self::Tool(t.deep_reflect()),
            Self::Weapon(w) => Self::Weapon(w.deep_reflect()),
            Self::Primordial(p) => Self::Primordial(p.deep_reflect()),
            Self::Consumable(c) => Self::Consumable(c.deep_reflect()),
        }
    }
}

impl Storage for Item {
    fn can_hold(&self, item: &Item) -> Result<(), StorageQueryError> {
        match self {
            Self::Container(c) => {
                if let Item::Container(other) = item {
                    if c.rank() < other.rank() {
                        return Err(StorageQueryError::InvalidHierarchy);
                    }
                }
                c.can_hold(item)
            }
            _ => Err(StorageQueryError::NotContainer)
        }
    }

    fn max_space(&self) -> StorageSpace {
        match self {
            Self::Container(c) => c.max_space(),
            _ => 0
        }
    }

    fn required_space(&self) -> StorageSpace {
        match self {
            Self::Container(c) => c.required_space(),
            _ => self.size()
        }
    }

    fn space(&self) -> StorageSpace {
        match self {
            Self::Container(c) => c.space(),
            _ => 0
        }
    }

    fn try_insert(&mut self, item: Item) -> Result<(), container::StorageError> {
        match self {
            Self::Container(c) => c.try_insert(item),
            _ => Err(container::StorageError::NotContainer(item))
        }
    }

    fn contains(&self, id: &str) -> bool {
        match self {
            Self::Container(c) => c.contains(id),
            _ => false
        }
    }

    fn peek_at(&self, id: &str) -> Option<&Item> {
        match self {
            Self::Container(c) => c.peek_at(id),
            _ => None
        }
    }

    fn take(&mut self, id: &str) -> Option<Item> {
        match self {
            Self::Container(c) => c.take(id),
            _ => None
        }
    }

    fn take_by_name(&mut self, id: &str) -> Option<Item> {
        match self {
            Self::Container(c) => c.take_by_name(id),
            _ => None,
        }
    }

    fn find_id_by_name(&self, name: &str) -> Option<String> {
        match self {
            Self::Container(c) => c.find_id_by_name(name),
            _ => None
        }
    }

    fn eject_all(&mut self) -> Option<Vec<Item>> {
        match self {
            Self::Container(v) => v.eject_all(),
            _ => None
        }
    }
}

impl Describable for Item {
    fn desc<'a>(&'a self) -> &'a str {
        match self {
            Self::Container(v) |
            Self::Corpse(v)    => v.desc(),
            Self::Key(v) => v.desc(),
            Self::Primordial(v) => v.desc(),
            Self::Tool(v) => v.desc(),
            Self::Weapon(v) => v.desc(),
            Self::Consumable(v) => v.desc(),
        }
    }
}

impl DescribableMut for Item {
    fn set_desc(&mut self, text: &str) -> bool {
        match self {
            Self::Corpse(v)    |
            Self::Container(v) => v.set_desc(text),
            Self::Key(v) => v.set_desc(text),
            Self::Primordial(v) => v.set_desc(text),
            Self::Tool(v) => v.set_desc(text),
            Self::Weapon(v) => v.set_desc(text),
            Self::Consumable(v) => v.set_desc(text),
        }
    }
}

impl Itemized for Item {
    fn size(&self) -> StorageSpace {
        match self {
            Self::Consumable(v) => v.size(),
            Self::Container(v) |
            Self::Corpse(v)    => v.size(),
            Self::Key(v) => v.size(),
            Self::Primordial(v) => v.size(),
            Self::Tool(v) => v.size(),
            Self::Weapon(v) => v.size(),
        }
    }
}

impl ItemizedMut for Item {
    fn set_size(&mut self, sz: StorageSpace) -> bool {
        match self {
            Self::Consumable(v) => v.set_size(sz),
            Self::Container(v) => v.set_size(sz),
            Self::Key(v) => v.set_size(sz),
            Self::Primordial(v) => v.set_size(sz),
            Self::Tool(v) => v.set_size(sz),
            Self::Weapon(v) => v.set_size(sz),
            Self::Corpse(_) => false,
        }
    }
}

impl StorageMut for Item {
    fn set_max_space(&mut self, sz: StorageSpace) -> bool {
        match self {
            Self::Container(v) => v.set_max_space(sz),
            Self::Primordial(v) => v.set_max_space(sz),
            _ => false
        }
    }
}

impl Metamorphize for Item {
    fn metamorph(self) -> Item {
        match self {
            Self::Primordial(v) => v.metamorph(),
            _ => self
        }
    }
}

#[async_trait]
impl Tickable for Item {
    async fn tick(&mut self) -> bool {
        match self {
            Self::Consumable(c) => c.tick().await,
            Self::Container(c)  |
            Self::Corpse(c)     => c.tick().await,
            Self::Primordial(c)   => c.tick().await,
            _ => false
        }
    }
}

#[cfg(test)]
mod item_tests {
    use crate::item::container::variants::ContainerVariantType;

    use super::*;

    #[test]
    fn test_vessel_hierarchy_and_volume() {
        let _ = env_logger::try_init();
        // 1. GENESIS: Create the World-Space (Room) and the Player
        //let mut room = ContainerVariant::new(ContainerVariantType::Room);
        let mut player_inv = ContainerVariant::new(ContainerVariantType::PlayerInventory);
        
        // 2. MATTER: Create a Backpack and a "Heavy" Key
        let mut backpack = ContainerVariant::new(ContainerVariantType::Backpack);
        // log::debug!("Backpack size: {}", backpack.required_space());
        let key = Item::Key(TemporaryStructToAppeaseAnalyzerDuringWIP {
            id: "iron-key-01".to_string(),
            title: "Heavy Iron Key".to_string(),
            owner: Owner::no_one(),
        });
        // log::debug!("Key size: {}", key.required_space());

        // 3. THE HIERARCHY LAW: Try to put the Player inv in the Backpack (Should fail)
        let result = backpack.try_insert(player_inv.reflect()); 
        log::debug!("{result:?}");
        assert!(matches!(result, Err(StorageError::InvalidHierarchy(_))));

        // 4. THE VOLUME LAW: Stomping the flat matter
        // Put the Key (size 1) into the Backpack (max 30).
        backpack.try_insert(key).expect("Key should fit in backpack");
        
        // 5. THE RECURSIVE RELAY: Put the full Backpack into the Player Inventory
        // Backpack size (2) + Key size (1) = 3 total units taken from Player (50).
        player_inv.try_insert(backpack).expect("Backpack should fit in player inventory");

        if let Item::Container(v) = &player_inv {
            assert_eq!(v.space(), 479); // The "Butcher" calculated the nested grit!
        }
    }
}
