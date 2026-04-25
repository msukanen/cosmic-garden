//! Persistent item blueprint library.

use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::sync::RwLock;

use crate::{identity::IdentityQuery, io::{blueprint_lib_fp, entity_lib_fp, help_lib_fp}, item::{BlueprintLibrary, container::Storage}, mob::spawn_lib::EntityLibrary, thread::{SystemSignal, add_item_to_lnf, signal::{SigReceiver, SignalSenderChannels, SpawnType}}, traits::Reflector, util::HelpLibrary, world::World};

lazy_static! {
    pub(crate) static ref BP_LIBRARY: Arc<RwLock<BlueprintLibrary>> = Arc::new(RwLock::new(BlueprintLibrary::default()));

    pub(crate) static ref HELP_LIBRARY: Arc<RwLock<HelpLibrary>> = Arc::new(RwLock::new(HelpLibrary::default()));
    pub(crate) static ref ENT_BP_LIBRARY: Arc<RwLock<EntityLibrary>> = Arc::new(RwLock::new(EntityLibrary::default()));
}

#[cfg(test)]
#[macro_export]
macro_rules! get_operational_mock_librarian {
    ($ch:ident, $w:ident) => {
        tokio::spawn( crate::thread::librarian(($ch.out.clone(), $ch.recv.librarian), $w.clone()))
    };
}

/// 
/// Librarian wake up.
/// 
/// This thread keeps the world's documents nice and tidy.
/// 
pub async fn librarian((out, mut incoming): (SignalSenderChannels, SigReceiver), world: Arc<RwLock<World>>) {
    // Bootstrap/load blueprints.
    log::info!("Library establishing… blueprints @ '{}'", blueprint_lib_fp().display());
    if let Err(e) = BlueprintLibrary::load_or_bootstrap().await {
        // Halt the printing press!!!
        log::error!("FAIL: Library in fire!!! {e:?}");
        return ;
    }

    // Bootstrap/load help files.
    log::info!("Library establishing… helpful documents @ '{}'", help_lib_fp().display());
    if let Err(e) = HelpLibrary::load_or_bootstrap().await {
        // Shucks! The documents are in fire!
        log::error!("Help! The help system is in distress! {e:?}");
        return ;
    }

    // Bootstrap/load entity blueprints.
    log::info!("Library establishing… biology @ '{}'", entity_lib_fp().display());
    if let Err(e) = EntityLibrary::load_or_bootstrap().await {
        // Aaah! The zoo is escaping!
        log::error!("Uwah! The zoo is in chaos! {e:?}");
        return ;
    }

    log::info!("Library didn't catch fire, yay.");
    let mut dusting_shelves_interval = tokio::time::interval(Duration::from_mins(10));
    let mut dusting_documents_interval = tokio::time::interval(Duration::from_mins(10));
    let mut species_catalogue_interval = tokio::time::interval(Duration::from_mins(10));
    
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

            _ = species_catalogue_interval.tick() => {
                log::trace!("Librarian checks through the species catalogue…");
                let mut lib = ENT_BP_LIBRARY.write().await;
                if let Err(e) = lib.save().await {
                    log::error!("\"What's going on here?!…\", a snag while saving the zoological documents: {e:?}");
                }
            }

            Some(sig) = incoming.recv() => match sig {
                SystemSignal::NewLibraryEntry => {
                    log::trace!("A new library entry? Let's see about that…");
                    if reorganize_library(&out).await {{
                        let phonebook = out.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(Duration::from_secs(30)).await;
                            if let Err(e) = phonebook.janitor.send(SystemSignal::ReindexLibrary) {
                                log::error!("Janitor is still not picking up the phone. Bah, he'll sort it out sooner or later… {e:?}");
                            }
                        });
                    }}
                }

                SystemSignal::NewBlueprintEntry => {
                    log::trace!("A new blueprint? Let's see what's that all about…");
                    out.janitor.send(SystemSignal::NewBlueprintEntry).ok();
                }

                SystemSignal::Shutdown => { break; }

                SystemSignal::Spawn { what: SpawnType::Item { id }, room_id } => {
                    let lock = BP_LIBRARY.read().await;
                    if let Some(found) = lock.get(&id) {
                        let item = found.reflect();
                        drop(lock);
                        if let Some(dest) = world.read().await.rooms.get(&room_id) {
                            let item_id = item.id().to_string();
                            let mut lock = dest.write().await;
                            if let Err(e) = lock.try_insert(item) {
                                drop(lock);
                                log::warn!("Item spawn failure. '{item_id}' sent to LnF.");
                                add_item_to_lnf(e).await;
                                continue;
                            }
                            log::info!("Librarian spawned '{item_id}' to '{room_id}'")
                        }
                    } else {
                        #[cfg(test)]{
                            log::debug!("Item ID '{id}' not found.");
                        }
                    }
                }

                // relay all other spawns but Item to Life.
                SystemSignal::Spawn { what, room_id } => {out.life.send(SystemSignal::Spawn { what, room_id }).ok();},
                _ => ()
            }
        }
    }

    BP_LIBRARY.write().await.save().await.ok();
    ENT_BP_LIBRARY.write().await.save().await.ok();
    HELP_LIBRARY.write().await.save().await.ok();
    log::info!("Librarian checking out.");
}

/// Reorganize the library, reindex, etc.
/// 
/// # Return
/// Anything to report?
async fn reorganize_library(outgoing: &SignalSenderChannels) -> bool {
    (*HELP_LIBRARY).write().await
        .check_new_docs()
        .rebuild_aliases();
    // ring the janitor …
    if let Err(e) = outgoing.janitor.send(SystemSignal::ReindexLibrary) {
        log::warn!("Janitor seems to be busy… I'll schedule call for later… {e:?}");
        return true;
    }
    false
}
