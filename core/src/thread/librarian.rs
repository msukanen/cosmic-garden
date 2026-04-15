//! Persistent item blueprint library.

use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::sync::{RwLock, mpsc};

use crate::{io::{blueprint_lib_fp, help_lib_fp}, item::BlueprintLibrary, thread::{SystemSignal, signal::SignalChannels}, util::HelpLibrary};

lazy_static! {
    pub static ref BP_LIBRARY: Arc<RwLock<BlueprintLibrary>> = Arc::new(RwLock::new(BlueprintLibrary::default()));

    pub static ref HELP_LIBRARY: Arc<RwLock<HelpLibrary>> = Arc::new(RwLock::new(HelpLibrary::default()));
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
