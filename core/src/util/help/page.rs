use std::collections::{HashMap, HashSet};

use convert_case::{Case, Casing};
use cosmic_garden_pm::{DescribableMut, IdentityMut};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{r#const::WORLD_ID, error::CgError, identity::IdentityQuery, io::{help_entry_fp, help_lib_fp}, serial::string_vec_to_bool_map, string::{Slugger, StrUuid, UNNAMED, Uuid, styling::maybe_plural}, thread::{SystemSignal, librarian::HELP_LIBRARY, signal::SignalChannels}, util::access::{Access, StrictAccess}};

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
    pub fn new(name: &str) -> Result<Self, CgError> {
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
    pub async fn save(&self) -> Result<(), CgError> {
        let nouuid = self.id().show_uuid(false);
        match toml::to_string_pretty(self) {
            Ok(contents) => {
                if let Err(e) = fs::write(help_entry_fp(&self.id()), contents).await {
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
    #[serde(skip)]
    new_docs: Vec<HelpPage>,
}

impl Default for HelpLibrary {
    fn default() -> Self {
        Self {
            world_id: UNNAMED.into(),
            id_stem: HashMap::new(),
            items: HashMap::new(),
            alias: HashMap::new(),
            new_docs: vec![],
        }
    }
}

impl HelpLibrary {
    // Load or bootstrap the help library.
    pub async fn load_or_bootstrap() -> Result<(), CgError> {
        // Bootstrap a brand new library if none exists yet.
        let Ok(mf) = fs::read_to_string(help_lib_fp()).await else {
            log::warn!("No papers, no ID? Ah well — preparing anyway…");

            let mut lock = (*HELP_LIBRARY).write().await;
            *lock = HelpLibrary::default();
            lock.world_id = WORLD_ID.as_str().into();

            let mut xfiles = HelpPage::new(&lock.world_id)?;
            xfiles.alias.insert("world".into());
            xfiles.alias.insert("cosmic-garden".into());
            xfiles.contents = format!("This entry was written by Cosmic Garden v{} bootstrap.\n", env!("CARGO_PKG_VERSION"));
            let nouuid = xfiles.id().no_uuid().to_string();
            lock.id_stem.insert(nouuid.clone(), true);
            lock.items.insert(nouuid, xfiles);

            lock.save().await?;
            log::info!("Library in place, just no documents yet.");
            return Ok(());
        };

        // We got either a bootstrapped or a loaded source in `mf`.
        let mut lib: HelpLibrary = serde_json::from_str(&mf)?;
        lib.world_id = WORLD_ID.as_str().into();
        for id in lib.id_stem.keys() {
            let item_path = help_entry_fp(id);
            match fs::read_to_string(&item_path).await {
                Ok(item_toml) => {
                    if let Ok(item) = toml::from_str(&item_toml) {
                        lib.items.insert(id.clone(), item);
                    }
                }
                Err(e) => log::warn!("Failed to fetch the '{id}' from {}: {e}", item_path.display())
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
        log::info!("Helpful documents for '{}' are now readable, live with {} entries.", WORLD_ID.as_str(), lock.items.len());

        Ok(())
    }

    /// Save the library and all the dirty entries.
    // (…especially the 'dirty' entries…)
    pub async fn save(&mut self) -> Result<(), CgError> {
        let contents = serde_json::to_string_pretty(&self)?;

        fs::write(help_lib_fp(), contents).await?;
        
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

    /// See if there's any new docs in new_docs queue.
    pub fn check_new_docs(&mut self) -> &mut Self {
        if self.new_docs.is_empty() { return self }

        for page in self.new_docs.iter_mut() {
            // mop up any potential UUID dust, normalize, and … Just in Case™
            page.id = page.id.show_uuid(false).to_lowercase();
            page.alias = page.alias.iter()
                .map(|alias| alias.show_uuid(false).to_lowercase())
                .collect::<HashSet<String>>();
            page.see_also = page.see_also.iter()
                .map(|sa| sa.show_uuid(false).to_lowercase())
                .collect::<HashSet<String>>();
            // there is a copy?
            if let Some(existing) = self.items.get(&page.id) {
                // remove old aliases that refer to it…
                for alias in &existing.alias {
                    self.alias.remove(alias);
                }
            }
            self.id_stem.insert(page.id.clone(), true);
            self.items.insert(page.id.clone(), page.clone());
        }

        self.new_docs = vec![];
        self
    }

    /// Reorganize the shelves.
    pub fn rebuild_aliases(&mut self) {
        log::trace!("Librarian is adjusting his glasses… re-indexing aliases.");

        self.alias.clear();

        for (id, page) in self.items.iter_mut() {
            let primary_id = id.clone().to_lowercase();
            self.id_stem.insert(primary_id.clone(), true);
            self.alias.insert(primary_id.clone(), primary_id.clone());
            
            for nick in &page.alias {
                self.alias.insert(nick.clone(), primary_id.clone());
            }
        }

        let alen = self.alias.len();
        let ilen = self.items.len();
        log::info!("Re-index complete. Librarian now knows {alen} path{} to {ilen} document{}.",
            maybe_plural(alen as i32), maybe_plural(ilen as i32));// the document number highly unlikely ever exceeds i32::MAX …
    }

    /// Shelve a document.
    /// 
    /// # Args
    /// - `item`— to use as blueprint (with UUID stripped).
    /// - `replace` old version if `true`.
    /// 
    /// # Return
    /// `true` if shelved for real.
    pub fn shelve(&mut self, entry: &HelpPage, system_ch: &SignalChannels) -> bool {
        // TODO content checking?
        self.new_docs.push(entry.clone());
        // poke the librarian but don't stand waiting…
        system_ch.librarian_tx.try_send(SystemSignal::NewLibraryEntry).ok();
        true
    }
}
