//! Items dwell here…

use cosmic_garden_pm::{IdentityMut, Itemized};
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{container::{Storage, StorageError, specs::StorageSpace, variants::ContainerVariant}, primordial::PrimordialItem}, string::{Describable, Uuid}, traits::Reflector};

pub mod owner;
pub mod container;
pub mod key;
pub mod primordial;
pub mod tool;
pub mod weapon;

pub trait Itemized {
    fn size(&self) -> StorageSpace;
}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut)]
pub struct TemporaryStructToAppeaseAnalyzerDuringWIP {
    pub(crate) id:String,
    pub(crate) title:String
}

impl Itemized for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn size(&self) -> StorageSpace {
        1
    }
}
impl Reflector for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn reflect(&self) -> Self {
        Self { id: self.id().re_uuid(), title: self.title.clone() }
    }
}

impl Describable for TemporaryStructToAppeaseAnalyzerDuringWIP {
    fn desc<'a>(&'a self) -> &'a str {
        "nothing to see here"
    }

    fn set_desc(&mut self, text: &str) -> bool {
        false
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, Itemized)]
/// Root [Item] types.
pub enum Item {
    Container(ContainerVariant),
    Weapon(TemporaryStructToAppeaseAnalyzerDuringWIP),
    Tool(TemporaryStructToAppeaseAnalyzerDuringWIP),
    Key(TemporaryStructToAppeaseAnalyzerDuringWIP),
    Consumable(TemporaryStructToAppeaseAnalyzerDuringWIP),
    Primordial(PrimordialItem),
}

impl Reflector for Item {
    fn reflect(&self) -> Self {
        match self {
            Self::Container(c) => Self::Container(c.reflect()),
            Self::Key(k) => Self::Key(k.reflect()),
            Self::Tool(t) => Self::Tool(t.reflect()),
            Self::Weapon(w) => Self::Weapon(w.reflect()),
            Self::Primordial(p) => Self::Primordial(p.reflect()),
            Self::Consumable(c) => Self::Consumable(c.reflect()),
        }
    }
}

impl Storage for Item {
    fn can_hold(&self, item: &Item) -> Result<bool, StorageError> {
        match self {
            Self::Container(c) => {
                if let Item::Container(other) = item {
                    if c.rank() < other.rank() {
                        return Err(StorageError::InvalidHierarchyQ);
                    }
                }
                c.can_hold(item)
            }
            _ => Ok(false)
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
}

impl Describable for Item {
    fn desc<'a>(&'a self) -> &'a str {
        match self {
            Self::Container(v) => v.desc(),
            Self::Key(v) => v.desc(),
            Self::Primordial(v) => v.desc(),
            Self::Tool(v) => v.desc(),
            Self::Weapon(v) => v.desc(),
            Self::Consumable(v) => v.desc(),
        }
    }

    fn set_desc(&mut self, text: &str) -> bool {
        match self {
            Self::Container(v) => v.set_desc(text),
            Self::Key(v) => v.set_desc(text),
            Self::Primordial(v) => v.set_desc(text),
            Self::Tool(v) => v.set_desc(text),
            Self::Weapon(v) => v.set_desc(text),
            Self::Consumable(v) => v.set_desc(text),
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
        let key = Item::Key(TemporaryStructToAppeaseAnalyzerDuringWIP {
            id: "iron-key-01".to_string(),
            title: "Heavy Iron Key".to_string(),
        });

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
            assert_eq!(v.space(), 47); // The "Butcher" calculated the nested grit!
        }
    }
}
