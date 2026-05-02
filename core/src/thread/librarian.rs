//! Librarian! She's cute, but does some heavy lifting if needed.

use std::{sync::Arc, time::Duration};

use tokio::sync::RwLock;

use crate::{
    help::{HelpLibrary, HelpPage}, identity::{IdentityQuery, uniq::{StrUuid, TryAttachUuid}}, item::{BlueprintLibrary, Item}, mob::{core::Entity, spawn_lib::EntityLibrary}, thread::{SystemSignal, add_item_to_lnf, signal::{SigReceiver, SignalSenderChannels, SpawnType}}, traits::Reflector, util::access::Access, world::World
};

#[cfg(test)]
#[macro_export]
macro_rules! get_operational_mock_librarian {
    ($ch:ident, $w:ident) => {
        tokio::spawn( crate::thread::librarian(($ch.out.clone(), $ch.recv.librarian), $w.clone()))
    };
}

#[derive(Debug, Clone, Copy)]
pub enum BlueprintType {
    Item,
    Mob,
}

struct Library {
    help: HelpLibrary,
    entity: EntityLibrary,
    bp: BlueprintLibrary,
}

#[cfg(test)]
mod librarian_test_cache {
    use std::sync::Arc;
    use tokio::sync::OnceCell;
    use crate::{help::HelpLibrary, item::BlueprintLibrary, mob::spawn_lib::EntityLibrary};

    pub static LIB_DATA_CACHE: OnceCell<Arc<(BlueprintLibrary, EntityLibrary, HelpLibrary)>> = OnceCell::const_new();
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
    #[cfg(test)]
    let shared_data = {
        librarian_test_cache::LIB_DATA_CACHE.get_or_init(|| async {
            let b = BlueprintLibrary::load_or_bootstrap().await.expect("Blueprint library in fire!");
            let h = HelpLibrary::load_or_bootstrap().await.expect("Help! Help in distress!");
            let e = EntityLibrary::load_or_bootstrap().await.expect("Zoo in chaos!");
            Arc::new((b,e,h))
        }).await.clone()
    };

    let mut lib = Library {
        #[cfg(test)]
        bp: shared_data.0.clone(),
        #[cfg(not(test))]
        bp: {
            // Bootstrap/load blueprints.
            use crate::io::blueprint_lib_fp;
            log::info!("Library establishing… blueprints @ '{}'", blueprint_lib_fp().display());
            match BlueprintLibrary::load_or_bootstrap().await {
                Err(e) => {
                    log::error!("FAIL: Library in fire!!! {e:?}");
                    return;
                }
                Ok(bp) => bp
            }
        },

        #[cfg(test)]
        help: shared_data.2.clone(),
        #[cfg(not(test))]
        help: {
            // Bootstrap/load help files.
            use crate::io::help_lib_fp;
            log::info!("Library establishing… helpful documents @ '{}'", help_lib_fp().display());
            match HelpLibrary::load_or_bootstrap().await {
                Err(e) => {
                    log::error!("Help! The help system is in distress! {e:?}");
                    return;
                }
                Ok(help) => help
            }
        },

        #[cfg(test)]
        entity: shared_data.1.clone(),
        #[cfg(not(test))]
        entity: {
            // Bootstrap/load entity blueprints.
            use crate::io::entity_lib_fp;
            log::info!("Library establishing… biology @ '{}'", entity_lib_fp().display());
            match EntityLibrary::load_or_bootstrap().await {
                Err(e) => {
                    log::error!("Uwah! The zoo is in chaos! {e:?}");
                    return;
                }
                Ok(entity) => entity
            }
        }
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

                SystemSignal::Spawn { what: SpawnType::Item { id }, room, .. } => {
                    if let Some(found) = lib.bp.get(&id) {
                        let item = found.reflect();
                        let r_id = room.id().await;
                        if let Some(dest) = world.read().await.get_room_by_id(&r_id).clone() {
                            let item_id = item.id().to_string();
                            let mut lock = dest.write().await;
                            if let Err(e) = lock.try_insert(item) {
                                drop(lock);
                                log::warn!("Item spawn failure. '{item_id}' sent to LnF.");
                                add_item_to_lnf(e).await;
                                continue;
                            }
                            log::info!("Librarian spawned '{item_id}' to '{r_id}'")
                        }
                    } else {
                        #[cfg(test)]{
                            log::debug!("Item ID '{id}' not found.");
                        }
                    }
                }

                // relay all other spawns but Item to Life.
                SystemSignal::Spawn { what, room, .. } => { out.life.send(SystemSignal::Spawn { what, room, reply: None }).ok(); },

                // help page request…
                SystemSignal::HelpRequest { page_id, access, bypass, out } => {
                    //log::debug!("Help request about '{page_id}'");
                    out.send(lib.help.get(&page_id, &access, bypass)).ok();
                }

                // entity BP request…
                SystemSignal::EntityBlueprintReq { id, out } => {
                    out.send(lib.entity.get(&id)).ok();
                }

                // item BP request…
                SystemSignal::ItemBlueprintReq { id, out } => {
                    out.send(lib.bp.get(&id).maybe_with_uuid()).ok();
                }

                // BP list request…
                SystemSignal::ListBlueprintReq { kind, term, out } => {
                    let keys = match kind {
                        BlueprintType::Item => lib.bp.keys(),
                        BlueprintType::Mob => lib.entity.keys(),
                    };

                    tokio::spawn(async move { search_coworker(keys, term, out) });
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
    let (oneshot, recv) = tokio::sync::oneshot::channel::<Option<HelpPage>>();
    if let Ok(_) = out.librarian.send(SystemSignal::HelpRequest { page_id: id.into(), access, bypass, out: oneshot }) {
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

/// Blueprint search coworker.
pub(crate) async fn search_coworker(list: Vec<String>, term: Option<String>, out: tokio::sync::oneshot::Sender<Vec<String>>) {
    let mut results: Vec<String> = if let Some(t) = term {
        let t = t.to_lowercase();
        list.into_iter()
            .filter(|id| {
                id.show_uuid(false).contains(&t)
            })
            .collect()
    } else {
        list
    };

    results.sort_unstable();
    out.send(results).ok();
}
