//! Persistent item blueprint library.

use std::{collections::HashMap, sync::Arc, time::Duration};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::{RwLock, mpsc}};

use crate::{r#const::WORLD_ID, identity::{IdentityMut, IdentityQuery}, io::{blueprint_entry_fp, blueprint_lib_fp, help_lib_fp}, item::Item, string::Uuid, thread::{SystemSignal, signal::SignalChannels}, util::HelpLibrary};

lazy_static! {
    pub static ref BP_LIBRARY: Arc<RwLock<BlueprintLibrary>> = Arc::new(RwLock::new(BlueprintLibrary::default()));

    pub static ref HELP_LIBRARY: Arc<RwLock<HelpLibrary>> = Arc::new(RwLock::new(HelpLibrary::default()));
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlueprintLibrary {
    world_id: String,
    #[serde(with = "string_vec_to_bool_map")]
    id_stem: HashMap<String, bool>,
    #[serde(default)]
    items: HashMap<String, Item>,
}

pub mod string_vec_to_bool_map {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeSeq};

    pub fn serialize<S>(map: &HashMap<String,bool>, s:S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut seq = s.serialize_seq(Some(map.len()))?;
        for id in map.keys() {
            seq.serialize_element(id)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(d:D) -> Result<HashMap<String,bool>, D::Error>
    where D: Deserializer<'de>
    {
        let ids = Vec::<String>::deserialize(d)?;
        Ok(ids.into_iter().map(|id|(id, false)).collect())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlueprintEntry {
    id: String,
    item: Item,
}

impl BlueprintLibrary {
    pub fn get(&self, id: &str) -> Option<Item> {
        self.items.get(id).cloned()
    }

    /// Shelve a (possibly new) blueprint, maybe `replace` old version while at it.
    /// 
    /// # Args
    /// - `item`— to use as blueprint (with UUID stripped).
    /// - `replace` old version if `true`.
    /// 
    /// # Return
    /// `true` if shelved for real.
    pub fn shelve(&mut self, item: &Item, replace: bool) -> bool {
        let id = item.id().no_uuid();

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

#[derive(Debug)]
pub enum BlueprintError {
    Io(std::io::Error),
    Json(serde_json::Error),
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
    pub async fn load_or_bootstrap() -> Result<(), BlueprintError> {
        // Library present? If no, make one.
        let Ok(mf) = fs::read_to_string(blueprint_lib_fp()).await else {
            log::warn!("No library established yet. Setting defaults…");
            let mut lock = (*BP_LIBRARY).write().await;
            lock.world_id = WORLD_ID.as_str().into();
            lock.id_stem = HashMap::new();
            lock.items = HashMap::new();
            lock.save().await?;
            log::info!("Library in place, just no blueprints yet.");
            return Ok(());
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

        let mut lock = (*BP_LIBRARY).write().await;
        *lock = lib;
        log::info!("Blueprint library for '{}' is now live with {} specimen.", WORLD_ID.as_str(), lock.items.len());

        Ok(())
    }

    /// Save the blueprint library and all the dirty marked entries.
    pub async fn save(&mut self) -> Result<(), BlueprintError> {
        let contents = serde_json::to_string_pretty(&self)?;
        fs::write(blueprint_lib_fp(), contents).await?;
        
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
                        log::trace!("Persisted specimen: '{id}'");
                    }
                }
                Err(e) => log::error!("FAIL: JSON serialization failed for '{id}': {e}")
            }
        }

        Ok(())
    }
}

/// 
/// Librarian wake up.
/// 
/// This thread keeps the world's documents nice and tidy.
/// 
pub async fn librarian((outgoing, mut incoming): (SignalChannels, mpsc::Receiver<SystemSignal>)) {
    log::info!("Library establishing… blueprints @ '{}'", blueprint_lib_fp().display());
    if let Err(e) = BlueprintLibrary::load_or_bootstrap().await {
        // Halt the printing press!!!
        log::error!("FAIL: Library in fire!!! {e:?}");
        return ;
    }
    log::info!("Library establishing… helpful documents @ '{}'", help_lib_fp().display());
    if let Err(e) = HelpLibrary::load_or_bootstrap().await {
        // Shucks! The documents are in fire!
        log::error!("Help! The help system is in distress! {e:?}");
        return ;
    }

    log::info!("Library didn't catch fire, yay.");
    let mut dusting_shelves_interval = tokio::time::interval(Duration::from_mins(10));
    let mut dusting_documents_interval = tokio::time::interval(Duration::from_mins(10));
    
    loop {
        tokio::select! {
            _ = dusting_shelves_interval.tick() => {
                log::trace!("Librarian sweeping the shelves pristine…");
                let mut lib = BP_LIBRARY.write().await;
                if let Err(e) = lib.save().await {
                    log::error!("\"FFS!\", a snag while saving blueprints: {e:?}");
                }
            }

            _ = dusting_documents_interval.tick() => {
                log::trace!("Librarian rummages through the documents…");
                let mut lib = HELP_LIBRARY.write().await;
                if let Err(e) = lib.save().await {
                    log::error!("\"Seriously!?…\", a snag while saving documents: {e:?}");
                }
            }

            Some(sig) = incoming.recv() => match sig {
                SystemSignal::NewLibraryEntry => {
                    if reorganize_library(&outgoing.janitor_tx).await {
                        {
                            let phonebook = outgoing.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_secs(30)).await;
                                if let Err(e) = phonebook.janitor_tx.send(SystemSignal::ReindexLibrary).await {
                                    log::error!("Janitor is still not picking up the phone. Bah, he'll sort it out sooner or later… {e:?}");
                                }
                            });
                        }
                    }
                },
                _ => ()
            }
        }
    }
}

/// Reorganize the library, reindex, etc.
/// 
/// # Return
/// Anything to report?
async fn reorganize_library(janitor_tx: &mpsc::Sender<SystemSignal>) -> bool {
    (*HELP_LIBRARY).write().await
        .check_new_docs()
        .rebuild_aliases();
    // ring the janitor …
    if let Err(e) = janitor_tx.send(SystemSignal::ReindexLibrary).await {
        log::warn!("Janitor seems to be busy… I'll schedule call for later… {e:?}");
        return true;
    }
    false
}
