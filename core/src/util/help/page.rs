use std::collections::{HashMap, HashSet};

use convert_case::{Case, Casing};
use cosmic_garden_pm::{DescribableMut, IdentityMut};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{identity::{IdError, IdentityQuery}, io::WORLD_ID, io_thread::SAVE_HELP_ASAP, library::{HELP_LIBRARY, HELP_PATH, string_vec_to_bool_map}, string::{Slugger, UNNAMED, Uuid}, util::access::{Access, StrictAccess}};

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, DescribableMut)]
pub struct HelpPage {
    /// internal ID
    id: String,
    /// Title of the document.
    title: String,
    /// Who can access this doc?
    #[serde(default)]
    pub(crate) access: Option<StrictAccess>,
    /// alias / also-known-as
    pub(crate) alias: HashSet<String>,
    /// What other pages are referred to, "see also".
    #[serde(default)]
    pub(crate) see_also: HashSet<String>,
    #[description(desc)]
    contents: String,
    /// Phrases to confuse anyone who doesn't have rights to read this particular page.
    #[serde(default)]
    pub(crate) obfuscation: Option<Vec<String>>,
}

impl HelpPage {
    /// Can `access` the page?
    pub fn can_access(&self, access: &Access) -> bool {
        let Some(dacc) = &self.access else { return true };
        match dacc {
            StrictAccess::Admin => matches!(access, Access::Admin),
            StrictAccess::Builder => matches!(access, Access::Admin|Access::Builder),
            StrictAccess::AnyBuilder => matches!(access, Access::Admin|Access::Builder|Access::Player { builder: true,.. }),
            _ => true
        }
    }

    /// Produce new document with default values.
    pub fn new(name: &str) -> Result<Self, IdError> {
        // see that the name will survive as an ID…
        let name = name.as_id()?;
        let alias = name.clone();
        let name = name.re_uuid();// everything in the 'verse has UUID (almost everything…)
        Ok(Self {
            id: name.clone(),
            title: alias.to_case(Case::Title),
            access: None,
            see_also: HashSet::new(),
            contents: String::from("To start, use <c yellow>'desc ='</c>, followed by whatever comes to mind."),
            alias: {
                let mut h = HashSet::new();
                h.insert(alias.to_lowercase());
                h
            },
            obfuscation: None
        })
    }

    /// Save me!
    pub async fn save(&self) -> Result<(), HelpSystemError> {
        let nouuid = self.id().no_uuid();
        let path = format!("{}/{}/{nouuid}.help", HELP_PATH.display(), WORLD_ID.as_str());
        match toml::to_string_pretty(self) {
            Ok(contents) => {
                if let Err(e) = fs::write(path, contents).await {
                    log::error!("FAILURE: could not write '{nouuid}' onto disk: {e}");
                    return Err(e.into());
                } else {
                    log::trace!("Filed document: '{nouuid}'");
                }
            }
            Err(e) => log::error!("FAIL: TOMLing of '{nouuid}' failed: {e}")
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelpLibrary {
    world_id: String,
    #[serde(with = "string_vec_to_bool_map")]
    id_stem: HashMap<String, bool>,
    //             ID,     entry
    items: HashMap<String, HelpPage>,

    // runtime sorted:
    #[serde(skip)]
    //             human,  ID
    alias: HashMap<String, String>,
}

impl Default for HelpLibrary {
    fn default() -> Self {
        Self {
            world_id: UNNAMED.into(),
            id_stem: HashMap::new(),
            items: HashMap::new(),
            alias: HashMap::new()
        }
    }
}

#[derive(Debug)]
pub enum HelpSystemError {
    Io(std::io::Error),
    Json(serde_json::Error),
    TomlDe(toml::de::Error),
    TomlSer(toml::ser::Error),
    IdError(IdError),
}

impl From<std::io::Error> for HelpSystemError { fn from(value: std::io::Error) -> Self { Self::Io(value) }}
impl From<serde_json::Error> for HelpSystemError { fn from(value: serde_json::Error) -> Self { Self::Json(value) }}
impl From<toml::de::Error> for HelpSystemError { fn from(value: toml::de::Error) -> Self { Self::TomlDe(value) }}
impl From<toml::ser::Error> for HelpSystemError { fn from(value: toml::ser::Error) -> Self { Self::TomlSer(value) }}
impl From<IdError> for HelpSystemError { fn from(value: IdError) -> Self { Self::IdError(value) }}

impl HelpLibrary {
    pub async fn load_or_bootstrap() -> Result<(), HelpSystemError> {
        let world_id = WORLD_ID.as_str();
        fs::create_dir_all(&format!("{}/{}", HELP_PATH.display(), world_id)).await?;
        let Ok(mf) = fs::read_to_string(&format!("{}/{}.library", HELP_PATH.display(), world_id)).await else {
            log::warn!("No papers, no ID? Ah well — preparing anyway…");
            let mut lock = (*HELP_LIBRARY).write().await;
            *lock = HelpLibrary::default();
            lock.world_id = world_id.into();

            let mut xfiles = HelpPage::new(world_id)?;
            xfiles.alias.insert("world".into());
            xfiles.alias.insert("cosmic-garden".into());
            xfiles.contents = format!("This entry was written by Cosmic Garden v{} bootstrap.", env!("CARGO_PKG_VERSION"));
            let nouuid = xfiles.id().no_uuid();
            lock.id_stem.insert(nouuid.clone(), true);
            lock.items.insert(nouuid, xfiles);

            lock.save().await?;
            log::info!("Library in place, just no documents yet.");
            return Ok(());
        };

        let mut lib: HelpLibrary = serde_json::from_str(&mf)?;
        lib.world_id = world_id.into();
        for id in lib.id_stem.keys() {
            let item_path = format!("{}/{}/{}.help", HELP_PATH.display(), world_id, id);
            match fs::read_to_string(&item_path).await {
                Ok(item_toml) => {
                    if let Ok(item) = toml::from_str(&item_toml) {
                        lib.items.insert(id.clone(), item);
                    }
                }
                Err(e) => log::warn!("Failed to fetch the '{id}' from {item_path}: {e}")
            }
        }

        // erase failed stems from library
        lib.id_stem.retain(|id,_| lib.items.contains_key(id));

        // some documents sorting…
        for (id, page) in &lib.items {
            // main, primary alias for Speedy Gonzales…
            lib.alias.insert(id.clone(), id.clone());
            for nick in &page.alias {
                lib.alias.insert(nick.to_lowercase(), id.clone());
            }
        }

        let mut lock = (*HELP_LIBRARY).write().await;
        *lock = lib;
        log::info!("Helpful documents for '{world_id}' are now readable, live with {} entries.", lock.items.len());

        Ok(())
    }

    pub async fn save(&mut self) -> Result<(), HelpSystemError> {
        let contents = serde_json::to_string_pretty(&self)?;
        fs::write(&format!("{}/{}.library", HELP_PATH.display(), WORLD_ID.as_str()), contents).await?;
        
        for (id, dirty) in self.id_stem.iter_mut() {
            if !*dirty { continue; }
            
            let Some(item) = self.items.get(id) else {
                log::error!("Could not find '{id}' in 'items'!");
                continue;
            };
            let _ = item.save().await;
        }

        Ok(())
    }

    /// Get a document, maybe…
    pub fn get(&self, id: &str, access: &Access, bypass_access: bool) -> Option<HelpPage> {
        let query = id.to_lowercase();
        if let Some(actual) = self.alias.get(&query) {
            if let Some(page) = self.items.get(actual) {
                if bypass_access || page.can_access(access) {
                    return page.clone().into();
                }
            }
        }
        None
    }

    /// Shelve a document.
    /// 
    /// # Args
    /// - `item`— to use as blueprint (with UUID stripped).
    /// - `replace` old version if `true`.
    /// 
    /// # Return
    /// `true` if shelved for real.
    pub fn shelve(&mut self, entry: &HelpPage) -> bool {
        // TODO content checking?
        self.id_stem.insert(entry.id().no_uuid(), true);
        self.items.insert(entry.id().no_uuid(), entry.clone());
        true
    }
}
