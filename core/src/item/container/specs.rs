//! Container specs themselves.

use std::collections::HashMap;

use cosmic_garden_pm::{IdentityMut, Itemized};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{identity::IdentityQuery, item::{Item, container::Storage, Itemized as _}, string::{UNNAMED, Uuid}};

pub type StorageSpace = u16;

lazy_static! {
    pub(super) static ref DEFAULT_BACKPACK_SPEC: ContainerSpec = ContainerSpec {
        id: "backpack".with_uuid(),
        name: "backpack".into(),
        contents: HashMap::new(),
        max_space: 30,
        size: 2,
    };

    pub(super) static ref DEFAULT_POUCH_SPEC: ContainerSpec = ContainerSpec {
        id: "pouch".with_uuid(),
        name: "pouch".into(),
        contents: HashMap::new(),
        max_space: 10,
        size: 1,
    };

    pub(super) static ref DEFAULT_PLR_INV_SPEC: ContainerSpec = ContainerSpec {
        id: "player-inventory".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 50,
        size: 0,
    };

    pub(super) static ref DEFAULT_ROOM_SPACE_SPEC: ContainerSpec = ContainerSpec {
        id: "room-space".with_uuid(),
        name: UNNAMED.into(),
        contents: HashMap::new(),
        max_space: 1_000,
        size: 0,
    };
}

#[derive(Debug, Deserialize, Serialize, IdentityMut, Itemized)]
pub struct ContainerSpec {
    id: String,
    #[identity(title)]
    name: String,
    contents: HashMap<String, Item>,
    max_space: StorageSpace,
    size: StorageSpace,
}

impl From<&ContainerSpec> for ContainerSpec {
    fn from(value: &ContainerSpec) -> Self {
        Self {
            id: value.id().re_uuid(),
            name: value.name.clone(),
            contents: HashMap::new(),
            max_space: value.max_space,
            size: value.size,
        }
    }
}

impl ContainerSpec {
    fn contents_size(&self) -> StorageSpace {
        let mut sz = 0;
        for x in self.contents.values() {
            sz += x.required_space()
        }
        sz
    }
}

impl Storage for ContainerSpec {
    fn can_hold(&self, item: &Item) -> bool {
        item.required_space() as usize
        + self.contents_size() as usize
        <= self.max_space as usize
    }

    fn max_space(&self) -> StorageSpace {
        self.max_space
    }
    
    fn required_space(&self) -> StorageSpace {
        self.size() + self.contents_size()
    }
    
    fn space(&self) -> StorageSpace {
        self.max_space - self.contents_size()
    }
}
