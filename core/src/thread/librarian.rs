//! Persistent item blueprint library.

use std::{sync::Arc, time::Duration};

use tokio::sync::RwLock;

use crate::{identity::IdentityQuery, io::{blueprint_lib_fp, entity_lib_fp, help_lib_fp}, item::{BlueprintLibrary, Item}, mob::{core::Entity, spawn_lib::EntityLibrary}, thread::{SystemSignal, add_item_to_lnf, signal::{SigReceiver, SignalSenderChannels, SpawnType}}, traits::Reflector, util::{HelpLibrary, HelpPage, access::Access}, world::World};

#[cfg(test)]
#[macro_export]
macro_rules! get_operational_mock_librarian {
    ($ch:ident, $w:ident) => {
        tokio::spawn( crate::thread::librarian(($ch.out.clone(), $ch.recv.librarian), $w.clone()))
    };
}

struct Library {
    help: HelpLibrary,
    entity: EntityLibrary,
    bp: BlueprintLibrary,
}

/// 
/// Librarian wake up.
/// 
/// This thread keeps the world's documents nice and tidy.
/// 
pub async fn librarian(
        (out, mut incoming): (SignalSenderChannels, SigReceiver),
        world: Arc<RwLock<World>>,
) {
    // Bootstrap/load blueprints.
    log::info!("Library establishing… blueprints @ '{}'", blueprint_lib_fp().display());
    let bp = BlueprintLibrary::load_or_bootstrap().await;
    if let Err(e) = bp {
        // Halt the printing press!!!
        log::error!("FAIL: Library in fire!!! {e:?}");
        return ;
    }

    // Bootstrap/load help files.
    log::info!("Library establishing… helpful documents @ '{}'", help_lib_fp().display());
    let help = HelpLibrary::load_or_bootstrap().await;
    if let Err(e) = help {
        // Shucks! The documents are in fire!
        log::error!("Help! The help system is in distress! {e:?}");
        return ;
    }

    // Bootstrap/load entity blueprints.
    log::info!("Library establishing… biology @ '{}'", entity_lib_fp().display());
    let entity = EntityLibrary::load_or_bootstrap().await;
    if let Err(e) = entity {
        // Aaah! The zoo is escaping!
        log::error!("Uwah! The zoo is in chaos! {e:?}");
        return ;
    }

    let mut lib = Library {
        bp: bp.unwrap(),
        help: help.unwrap(),
        entity: entity.unwrap(),
    };
    log::info!("Library didn't catch fire, yay.");
    let mut dusting_shelves_interval = tokio::time::interval(Duration::from_mins(10));
    let mut dusting_documents_interval = tokio::time::interval(Duration::from_mins(10));
    let mut species_catalogue_interval = tokio::time::interval(Duration::from_mins(10));
    
    loop {
        tokio::select! {
            _ = dusting_shelves_interval.tick() => {
                log::trace!("Librarian sweeping the shelves pristine…");
                if let Err(e) = lib.bp.save().await {
                    log::error!("\"FFS!\", a snag while saving blueprints: {e:?}");
                }
            }

            _ = dusting_documents_interval.tick() => {
                log::trace!("Librarian rummages through the documents…");
                if let Err(e) = lib.help.save().await {
                    log::error!("\"Seriously!?…\", a snag while saving documents: {e:?}");
                }
            }

            _ = species_catalogue_interval.tick() => {
                log::trace!("Librarian checks through the species catalogue…");
                if let Err(e) = lib.entity.save().await {
                    log::error!("\"What's going on here?!…\", a snag while saving the zoological documents: {e:?}");
                }
            }

            Some(sig) = incoming.recv() => match sig {
                SystemSignal::NewHelpEntry { entry, out } => {
                    log::trace!("A new library entry? Let's see about that…");
                    let shelved = lib.help.shelve(&entry);
                    if shelved {
                        reorganize_library(&mut lib.help).await;
                    }
                    out.send(shelved).ok();
                }

                SystemSignal::NewBlueprintEntry { entry, out } => {
                    log::trace!("A new blueprint? Let's see what's that all about…");
                    let shelved = lib.bp.shelve(entry, true);
                    lib.bp.save().await.ok();
                    out.send(shelved).ok();
                }

                SystemSignal::NewEntityEntry { entry } => {
                    log::trace!("A new thing for entity catalogue? Let's see it's all about…");
                    lib.entity.shelve(entry);
                    lib.entity.save().await.ok();
                }

                SystemSignal::Shutdown => { break; }

                SystemSignal::Spawn { what: SpawnType::Item { id }, room_id } => {
                    if let Some(found) = lib.bp.get(&id) {
                        let item = found.reflect();
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
                SystemSignal::Spawn { what, room_id } => { out.life.send(SystemSignal::Spawn { what, room_id }).ok(); },

                // help page request…
                SystemSignal::HelpRequest { page_id, access, bypass, out } => {
                    log::debug!("Help request about '{page_id}'");
                    out.send(lib.help.get(&page_id, &access, bypass)).ok();
                }

                // entity BP request…
                SystemSignal::EntityBlueprintReq { id, out } => {
                    out.send(lib.entity.get(&id)).ok();
                }
                _ => ()
            }
        }
    }

    lib.bp.save().await.ok();
    lib.entity.save().await.ok();
    lib.help.save().await.ok();
    log::info!("Librarian checking out.");
}

/// Reorganize the library, reindex, etc.
async fn reorganize_library(lib: &mut HelpLibrary) {
    lib
        .check_new_docs()
        .rebuild_aliases();
}

/// Helper to get an [Entity] blueprint.
/// 
/// # Args
/// - `id` of [Entity] blueprint.
/// - `out` going signal system…
pub async fn get_entity_blueprint(id: &str, out: &SignalSenderChannels) -> Option<Entity> {
    let (oneshot, recv) = tokio::sync::oneshot::channel::<Option<Entity>>();
    if let Ok(_) = out.librarian.send(SystemSignal::EntityBlueprintReq { id: id.into(), out: oneshot }) {
        if let Ok(reply) = recv.await {
            return reply
        }
    }
    None
}

/// Attempt to shelve an [Entity] blueprint.
/// 
/// # Args
/// - [`entity`][Entity] to persist as a blueprint.
pub async fn shelve_entity_blueprint(entity: &Entity, out: &SignalSenderChannels) {
    out.librarian.send(SystemSignal::NewEntityEntry { entry: entity.clone() }).ok();
}

/// Attempt to shelve an [Item] blueprint.
/// 
/// # Args
/// - `item` to persist as an [Item] blueprint.
/// 
/// # Returns
/// Success of persistance.
pub async fn shelve_item_blueprint(item: &Item, out: &SignalSenderChannels) -> bool {
    let (oneshot, recv) = tokio::sync::oneshot::channel::<bool>();
    if let Ok(_) = out.librarian.send(SystemSignal::NewBlueprintEntry { entry: item.clone(), out: oneshot }) {
        if let Ok(reply) = recv.await {
            return reply;
        }
    }
    false
}

/// Helper to get an [Item] blueprint.
/// 
/// # Args
/// - `id` of [Item] blueprint.
/// - `out` going signal system…
pub async fn get_item_blueprint(id: &str, out: &SignalSenderChannels) -> Option<Item> {
    let (oneshot, recv) = tokio::sync::oneshot::channel::<Option<Item>>();
    if let Ok(_) = out.librarian.send(SystemSignal::ItemBlueprintReq { id: id.into(), out: oneshot }) {
        if let Ok(reply) = recv.await {
            return reply
        }
    }
    None
}

/// Helper to get a [HelpPage].
/// 
/// # Args
/// - `id` of [HelpPage].
/// - `out`going signal system…
pub async fn get_help_page(id: &str, access: Access, bypass: bool, out: &SignalSenderChannels) -> Option<HelpPage> {
    log::debug!("Oneshot get_help_page …");
    let (oneshot, recv) = tokio::sync::oneshot::channel::<Option<HelpPage>>();
    if let Ok(_) = out.librarian.send(SystemSignal::HelpRequest { page_id: id.into(), access, bypass, out: oneshot }) {
        log::debug!("… got reply …");
        if let Ok(reply) = recv.await {
            return reply
        }
    }
    None
}

/// Attempt to shelve a [HelpPage].
/// 
/// # Args
/// - [`entry`][HelpPage] to shelve.
/// - `out`going signal system…
pub async fn shelve_help_page(entry: &HelpPage, out: &SignalSenderChannels) -> bool {
    let (oneshot, recv) = tokio::sync::oneshot::channel::<bool>();
    if let Ok(_) = out.librarian.send(SystemSignal::NewHelpEntry { entry: entry.clone(), out: oneshot }) {
        if let Ok(reply) = recv.await {
            return reply;
        }
    }
    false
}
