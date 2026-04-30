//! Mob/Entity blueprint library.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{r#const::WORLD_ID, error::CgError, identity::IdentityQuery, io::{entity_entry_fp, entity_lib_fp}, mob::core::Entity, serial::string_vec_to_bool_map, identity::uniq::StrUuid};

/// Entity (blueprint) library!
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntityLibrary {
    world_id: String,
    #[serde(with = "string_vec_to_bool_map")]
    pub id_stem: HashMap<String, bool>,
    #[serde(default, skip)]
    pub bps: HashMap<String, Entity>,
}

impl EntityLibrary {
    /// Load/bootstrap [EntityLibrary]
    pub(crate) async fn load_or_bootstrap() -> Result<EntityLibrary, CgError> {
        let Ok(mf) = fs::read_to_string(entity_lib_fp()).await else {
            log::warn!("No entity database of any sort yet. Making (an empty) one…");

            let mut lib = EntityLibrary::default();

            let basic_mobs_toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/bootstrap_mob.toml"));
            #[derive(Deserialize)]
            struct BasicMobs {
                mobs: Vec<Entity>,
            }
            let wrapper: BasicMobs = toml::from_str(basic_mobs_toml)?;
            for mob in wrapper.mobs {
                log::debug!(" EL-boot → '{}'", mob.id());
                lib.id_stem.insert(mob.id().to_string(), true);
                lib.bps.insert(mob.id().to_string(), mob);
            }
            lib.save().await?;

            return  Ok(lib)
        };

        // We got a loaded lib in `mf`
        let mut lib: EntityLibrary = serde_json::from_str(&mf)?;
        if lib.world_id != WORLD_ID.as_str() {
            log::warn!("Loading entities of a different world ('{}')… Adjusting accordingly…", lib.world_id);
        }
        lib.world_id = WORLD_ID.as_str().into();
        for (id, dirty) in lib.id_stem.iter_mut() {
            let item_path = entity_entry_fp(id);
            // log::info!("item_path = {}", item_path.display());
            match fs::read_to_string(&item_path).await {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(ent) => {
                            lib.bps.insert(id.clone(), ent);
                            *dirty = false;
                        },
                        Err(e) => {
                            log::error!("What's up with this '{}' {e:?}?!", id);
                        }
                    }
                }

                Err(e) => log::warn!("Failed to fetch the '{id}' from {}: {e}", item_path.display())
            }
        }

        // mop up stems that failed to load…
        lib.id_stem.retain(|id,_| lib.bps.contains_key(id));
        let count = lib.id_stem.len();
        let plural = if count == 1 {"y"} else {"ies"};
        log::info!("Entity catalogue for '{}' established with {count} entr{plural}.", WORLD_ID.as_str());

        Ok(lib)
    }

    /// Save entity library and all the registered, modified entries.
    pub(crate) async fn save(&mut self) -> Result<(), CgError> {
        let contents = serde_json::to_string_pretty(&self)?;
        fs::write(entity_lib_fp(), contents).await?;

        for (id, dirty) in self.id_stem.iter_mut() {
            if !*dirty { continue; }

            let Some(mob) = self.bps.get(id) else {
                log::error!("Could not find '{id}' in 'bps'");
                continue;
            };

            if let Ok(_) = mob.save_bp().await {
                *dirty = false;
            }
        }

        Ok(())
    }

    /// Get a copy of an [Entity] blueprint from cold storage, if exists.
    pub fn get(&self, id: &str) -> Option<Entity> {
        self.bps.get(id.show_uuid(false)).cloned()
    }

    /// Shelve a new [Entity] blueprint.
    pub fn shelve(&mut self, bp: Entity) {
        let id = bp.id().show_uuid(false).to_string();
        self.id_stem.insert(id.clone(), true);
        self.bps.insert(id, bp);
    }
}

impl Default for EntityLibrary {
    fn default() -> Self {
        EntityLibrary {
            world_id: WORLD_ID.as_str().to_string(),
            id_stem: HashMap::new(),
            bps: HashMap::new()
        }
    }
}
