//! Blueprint library.

use std::{collections::HashMap, fmt::Display};
use crate::{
    r#const::WORLD_ID,
    identity::{IdentityMut, IdentityQuery},
    io::{blueprint_entry_fp, blueprint_lib_fp},
    item::Item,
    serial::string_vec_to_bool_map,
    identity::uniq::{StrUuid, Uuid}
};

use serde::{Deserialize, Serialize};
use tokio::fs;
#[derive(Debug, Deserialize, Serialize)]
pub struct BlueprintLibrary {
    world_id: String,
    #[serde(with = "string_vec_to_bool_map")]
    id_stem: HashMap<String, bool>,
    #[serde(default, skip)]
    items: HashMap<String, Item>,
}

impl BlueprintLibrary {
    /// Try get blueprint with `id` from library.
    pub fn get(&self, id: &str) -> Option<Item> {
        self.items.get(id.show_uuid(false)).cloned()
    }

    /// Shelve a (possibly new) blueprint, maybe `replace` old version while at it.
    /// 
    /// # Args
    /// - `item`— to use as blueprint (with UUID stripped).
    /// - `replace` old version if `true`.
    /// 
    /// # Return
    /// `true` if shelved for real.
    pub fn shelve(&mut self, item: Item, replace: bool) -> bool {
        let id = item.id().no_uuid().to_string();

        if self.id_stem.contains_key(&id) && !replace {
            return false;
        }

        let mut bp_item = item.clone();
        *(bp_item.id_mut()) = id.clone();
        self.items.insert(id.clone(), bp_item);
        let existed = self.id_stem.insert(id.clone(), true);

        log::info!("Shelved specimen '{id}'{}.", if existed.is_some() && replace {", overriding old version"} else {""});
        true
    }
}

/// Various blueprint related errors…
#[derive(Debug)]
pub enum BlueprintError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl Display for BlueprintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "BP I/O {e:?}"),
            Self::Json(e) => write!(f, "BP JSON {e:?}"),
        }
    }
}
impl Default for BlueprintLibrary {
    fn default() -> Self {
        Self { world_id: "foobar".into(), id_stem: HashMap::new(), items: HashMap::new() }
    }
}

impl From<std::io::Error> for BlueprintError { fn from(value: std::io::Error) -> Self { Self::Io(value)}}
impl From<serde_json::Error> for BlueprintError { fn from(value: serde_json::Error) -> Self { Self::Json(value)}}

impl BlueprintLibrary {
    /// Load or bootstrap the blueprint library.
    pub async fn load_or_bootstrap() -> Result<BlueprintLibrary, BlueprintError> {
        // Library present? If no, make one.
        let Ok(mf) = fs::read_to_string(blueprint_lib_fp()).await else {
            log::warn!("No library established yet. Setting defaults…");
            let mut lib = BlueprintLibrary::default();
            lib.world_id = WORLD_ID.as_str().into();
            lib.id_stem = HashMap::new();
            lib.items = HashMap::new();
            lib.save().await?;
            log::info!("Library in place, just no blueprints yet.");
            return Ok(lib);
        };

        // Load the library.
        let mut lib: BlueprintLibrary = serde_json::from_str(&mf)?;
        lib.world_id = WORLD_ID.as_str().into();
        for id in lib.id_stem.keys() {
            match fs::read_to_string(blueprint_entry_fp(&id)).await {
                Ok(item_json) => {
                    if let Ok(item) = serde_json::from_str::<Item>(&item_json) {
                        lib.items.insert(id.clone(), item);
                    }
                }
                Err(e) => log::warn!("Failed to hydrate blueprint '{id}': {e}")
            }
        }

        // erase failed stems from library
        lib.id_stem.retain(|id,_| lib.items.contains_key(id));
        log::info!("Blueprint library for '{}' is now live with {} specimen.", WORLD_ID.as_str(), lib.items.len());
        Ok(lib)
    }

    /// Save the blueprint library and all the dirty marked entries.
    pub async fn save(&mut self) -> Result<(), BlueprintError> {
        let contents = serde_json::to_string_pretty(&self)?;
        fs::write(blueprint_lib_fp(), contents).await?;
        
        let mut any_saved = false;
        for (id, dirty) in self.id_stem.iter_mut() {
            if !*dirty { continue; }
            
            let Some(item) = self.items.get(id) else {
                log::error!("Could not find '{id}' in 'items'!");
                continue;
            };

            match serde_json::to_string_pretty(item) {
                Ok(contents) => {
                    if let Err(e) = fs::write(blueprint_entry_fp(&id), contents).await {
                        log::error!("FAILURE: could not write '{id}' onto disk: {e}");
                    } else {
                        *dirty = false;
                        any_saved = true;
                        log::trace!("Persisted specimen: '{id}'");
                    }
                }
                Err(e) => log::error!("FAIL: JSON serialization failed for '{id}': {e}")
            }
        }
        if any_saved {
            log::trace!("Blueprint library stored.");
        }
        Ok(())
    }
}

