use std::collections::{HashMap, HashSet};

use convert_case::{Case, Casing};
use cosmic_garden_pm::{DescribableMut, IdentityMut};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{r#const::WORLD_ID, error::CgError, identity::IdentityQuery, io::{help_entry_fp, help_lib_fp}, serial::string_vec_to_bool_map, string::{Slugger, StrUuid, UNNAMED, Uuid, styling::maybe_plural}, util::access::{Access, StrictAccess}};

#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut, DescribableMut)]
pub struct HelpPage {
    /// internal ID
    id: String,
    
    /// Title of the document.
    title: String,
    
    /// Who can access this doc?
    #[serde(default)]
    pub(crate) access: Option<StrictAccess>,

    #[serde(default)]
    /// alias / also-known-as
    pub(crate) alias: HashSet<String>,
    
    /// What other pages are referred to, "see also".
    #[serde(default)]
    pub(crate) see_also: HashSet<String>,
    
    #[description(desc)] contents: String,
    
    /// Phrases to confuse anyone who doesn't have rights to read this particular page.
    #[serde(default)]
    pub(crate) obfuscation: Option<Vec<String>>,
    
    /// Optional 'usage' entries. Meant mainly for commands, naturally.
    #[serde(default)]
    pub(crate) usage: Vec<String>,
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
        // namespaced?
        let (namespace, maybe_name) = name.split_once(':').unwrap_or((name, ""));
        let name = if maybe_name.is_empty() {
            namespace.as_id()?
        } else {
            let prefix = namespace.as_id()?;
            let suffix = maybe_name.as_id()?;
            format!("{prefix}-{suffix}")
        };

        let alias = name.show_uuid(false).to_string();
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
            obfuscation: None,
            usage: vec![]
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

    /// Get the usage as formatted string.
    pub fn usage(&self) -> String {
        let mut out = String::from("<c green>Usage:</c> ");
        for (it, text) in self.usage.iter().enumerate() {
            if it == 0 {
                out.push_str(&format!("{text}\n"));
            } else {
                out.push_str(&format!("       {text}\n"));
            }
        }
        out
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelpLibrary {
    world_id: String,
    #[serde(with = "string_vec_to_bool_map")]
    id_stem: HashMap<String, bool>,
    //             ID,     entry
    #[serde(skip, default)]
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
    pub async fn load_or_bootstrap() -> Result<HelpLibrary, CgError> {
        // Bootstrap a brand new library if none exists yet.
        let Ok(mf) = fs::read_to_string(help_lib_fp()).await else {
            log::warn!("No papers, no ID? Ah well — preparing anyway…");

            let mut lib = HelpLibrary::default();
            lib.world_id = WORLD_ID.as_str().into();

            let primordial_toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/bootstrap_help.toml"));
            #[derive(Deserialize)]
            struct PrimordialLexicon {
                help_pages: Vec<HelpPage>
            }
            let wrapper: PrimordialLexicon = toml::from_str(primordial_toml)?;
            for page in wrapper.help_pages {
                lib.id_stem.insert(page.id().to_lowercase(), true);
                lib.items.insert(page.id().to_lowercase(), page);
            }

            let mut xfiles = HelpPage::new(&lib.world_id)?;
            xfiles.alias.insert("world".into());
            xfiles.alias.insert("cosmic-garden".into());
            xfiles.contents = format!("This entry was written by Cosmic Garden v{} bootstrap.\n", env!("CARGO_PKG_VERSION"));
            let nouuid = xfiles.id().no_uuid().to_string();
            lib.id_stem.insert(nouuid.clone(), true);
            lib.items.insert(nouuid, xfiles);

            lib.save().await?;
            log::info!("Library in place, just no documents yet.");
            return Ok(lib);
        };

        // We got a loaded source in `mf`.
        let mut lib: HelpLibrary = serde_json::from_str(&mf)?;
        if lib.world_id != WORLD_ID.as_str() {
            log::warn!("Loading help entries of a different world ('{}')… Adjusting accordingly…", lib.world_id);
        }
        lib.world_id = WORLD_ID.as_str().into();
        for (id, dirty) in lib.id_stem.iter_mut() {
            let item_path = help_entry_fp(id);
            match fs::read_to_string(&item_path).await {
                Ok(item_toml) => {
                    if let Ok(item) = toml::from_str(&item_toml) {
                        lib.items.insert(id.clone(), item);
                        *dirty = false;
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

        log::info!("Helpful documents for '{}' are now readable, live with {} entries.", WORLD_ID.as_str(), lib.items.len());

        Ok(lib)
    }

    /// Save the library and all the dirty entries.
    // (…especially the 'dirty' entries…)
    pub async fn save(&mut self) -> Result<(), CgError> {
        let contents = serde_json::to_string_pretty(&self)?;

        fs::write(help_lib_fp(), contents).await?;
        
        let mut any_saved = false;
        for (id, dirty) in self.id_stem.iter_mut() {
            if !*dirty { continue; }
            
            let Some(item) = self.items.get(id) else {
                log::error!("Could not find '{id}' in 'items'!");
                continue;
            };
            if let Ok(_) = item.save().await {
                *dirty = false;
                any_saved = true;
            }
        }

        if any_saved {
            log::trace!("Library tucked onto disk.");
        }
        Ok(())
    }

    /// Get a document, maybe…
    /// 
    /// # Args
    /// - `id` of the document. May be namespaced with ':' as separator.
    /// - `access` rights of the querier.
    /// - `bypass_access` rights should be used *sparingly*…
    pub fn get(&self, id: &str, access: &Access, bypass_access: bool) -> Option<HelpPage> {
        log::trace!("Librarian is looking for… '{id}'…");
        fn actual_get(lib: &HelpLibrary, query: &str, access: &Access, bypass_access: bool) -> Option<HelpPage> {
            if let Some(actual) = lib.alias.get(query) {
                if let Some(page) = lib.items.get(actual) {
                    if bypass_access || page.can_access(access) {
                        return page.clone().into();
                    }
                }
            }
            None
        }

        let query = id.to_lowercase();
        let (namespace, actual) = query.as_str().split_once(':').unwrap_or((query.as_str(), ""));
        if !actual.is_empty() {
            let query = format!("{}-{}", namespace, actual);
            if let Some(page) = actual_get(self, &query, access, bypass_access) {
                return Some(page);
            }
        }

        actual_get(self, &query, access, bypass_access)
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
    pub fn shelve(&mut self, entry: &HelpPage) -> bool {
        log::trace!("Shelving {} …", entry.id());
        // TODO content checking?
        self.new_docs.push(entry.clone());
        true
    }
}
