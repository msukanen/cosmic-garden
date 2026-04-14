//! Disk I/O threading…

use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{sync::{RwLock, mpsc}, time};

use crate::{Cli, DATA_PATH, identity::IdentityQuery, item::Item, player::Player, room::Room, thread::{SystemSignal, signal::SignalChannels}, util::{HelpLibraryState, HelpPage}, world::World};

lazy_static! {
    pub static ref PLAYERS_TO_LOGOUT: Arc<RwLock<Vec<Arc<RwLock<Player>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref ROOMS_TO_SAVE: Arc<RwLock<Vec<Arc<RwLock<Room>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref SAVE_ASAP: Arc<RwLock<Vec<Arc<RwLock<Player>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref SAVE_HELP_ASAP: Arc<RwLock<Vec<Arc<RwLock<HelpPage>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref SAVE_HELP_ASAP_STATE: Arc<RwLock<HelpLibraryState>> = Arc::new(RwLock::new(HelpLibraryState::Stable));
    pub static ref LOST_AND_FOUND: Arc<RwLock<Vec<Item>>> = Arc::new(RwLock::new(Vec::new()));
}
pub const SAVE_ASAP_THRESHOLD: usize = 100;

/// Disk I/O thread thing.
/// 
/// `io_thread` handles all sorts of "janitorial" tasks from
/// autosaves and logouts to keeping the live world and disk
/// in (relative) sync.
/// 
pub(crate) async fn io_thread((outgoing, mut incoming): (SignalChannels, mpsc::Receiver<SystemSignal>), world: Arc<RwLock<World>>, args: Cli) {
    log::trace!("Firing up; DATA_PATH = '{}'", *DATA_PATH);

    let mut autosave_queue_interval = time::interval(Duration::from_secs(args.autosave_queue_interval.unwrap_or(300)));
    let mut logout_purge_interval = time::interval(Duration::from_secs(1));
    let mut world_save_interval = time::interval(Duration::from_mins(2));
    let mut room_save_interval = time::interval(Duration::from_secs(30));
    let mut save_asap_interval = time::interval(Duration::from_secs(2));
    let mut save_help_asap_interval = time::interval(Duration::from_secs(45));
    let mut lost_and_found_interval = time::interval(Duration::from_mins(2));

    loop {
        tokio::select! {
            // Logout handling.
            _ = logout_purge_interval.tick() => logout_purge().await,
            // Auto-saving the meningfully active Players.
            _ = autosave_queue_interval.tick() => autosave_queue(world.clone()).await,
            // Save players that need saving *now*.
            _ = save_asap_interval.tick() => save_asap().await,
            // Save the world, especially the whales!
            _ = world_save_interval.tick() => save_the_whales(world.clone()).await,
            // Save the modified [Room]s.
            _ = room_save_interval.tick() => room_save().await,
            // Handle lost and found items…
            _ = lost_and_found_interval.tick() => lost_and_found(world.clone()).await,
            // Anything in mailbox?
            Some(sig) = incoming.recv() => match sig {
                SystemSignal::Shutdown => break,
                SystemSignal::SaveWorld => save_the_whales(world.clone()).await,
                SystemSignal::LostAndFound => lost_and_found(world.clone()).await,
                SystemSignal::ReindexLibrary => save_help_asap().await,
                _ => ()
            }
        }
    }

    // Ok, time to close the shop.
}

/// Purge logged out players.
async fn logout_purge() {
    let players_to_save = {
        let mut qlock = (*PLAYERS_TO_LOGOUT).write().await;
        qlock.drain(..).collect::<Vec<_>>()
    };
    if players_to_save.is_empty() { return ; }
    
    log::info!("Saving {} disconnected player{}…", players_to_save.len(), if players_to_save.len() == 1 {""} else {"s"});
    let mut count = 0;
    for p in players_to_save {
        let p_id = {
            let p = p.read().await;
            p.id().to_string()
        };
        if let Err(e) = p.write().await.save().await {
            log::error!("Failed to save player '{p_id}': {e:?}")
        }
        count += 1;
        if count % 500 == 0 {
            log::debug!("{count} saves done.");
        }
    }
    log::info!("Disconnected player save cycle complete.");
}

/// Auto-save cycle.
async fn autosave_queue(world: Arc<RwLock<World>>) {
    let mut players_to_save = {
        let w = world.read().await;
        let mut p_arcs = vec![];
        for (_,p) in w.players_by_id.iter() {
            if p.read().await.actions_taken > 0 {
                p_arcs.push(p.clone());
            }
        }
        p_arcs
    };
    let any_saved = if !players_to_save.is_empty() {
        log::info!("Autosave cycle initiated.");
        true
    } else {false};
    for p in players_to_save.iter_mut() {
        let mut lock = p.write().await;
        if let Err(e) = lock.save().await {
            log::error!("Failed to save player '{}': {e:?}", lock.id());
        } else {
            lock.actions_taken = 0;
        }
    }
    if any_saved {
        log::info!("Autosave cycle complete.");
    }
}

/// ASAP save of a [Player].
async fn save_asap() {
    let players_to_save = {
        let mut qlock = (*SAVE_ASAP).write().await;
        qlock.drain(..).collect::<Vec<_>>()
    };
    if players_to_save.is_empty() { return; }
    
    log::info!("Saving {} hyper-active player{}…", players_to_save.len(), if players_to_save.len() == 1 {""} else {"s"});
    let mut fails = vec![];
    for p in players_to_save {
        let p_id = {
            let p = p.read().await;
            p.id().to_string()
        };

        if let Err(e) = p.write().await.save().await {
            log::error!("Failed to save player '{p_id}': {e:?}");
            fails.push(p.clone());
            continue;
        }

        p.write().await.actions_taken = 0;
    }
    if !fails.is_empty() {
        (*SAVE_ASAP).write().await.extend(fails);
    }
    log::info!("Hyper-active save cycle complete.");
}

/// World save cycle.
async fn save_the_whales(world: Arc<RwLock<World>>) {
    if let Err(e) = world.read().await.save().await {
        log::error!("World save failed?! {e:?}");
    } else {
        log::info!("World save cycle complete.");
    }
}

/// [Room(s)][Room] save cycle.
async fn room_save() {
    let rooms_to_save = {
        let mut qlock = (*ROOMS_TO_SAVE).write().await;
        qlock.drain(..).collect::<Vec<_>>()
    };
    if rooms_to_save.is_empty() { return ; }
    let mut rooms_to_requeue = vec![];
    for r in rooms_to_save {
        if let Err(e) = r.read().await.save().await {
            log::error!("Error saving room: '{e:?}'");
            rooms_to_requeue.push(r.clone());
        }
    }
    if !rooms_to_requeue.is_empty() {
        (*ROOMS_TO_SAVE).write().await.extend(rooms_to_requeue);
    }
}

/// Add an [Item] into Lost'n'Found queue.
pub async fn add_item_to_lnf<T>(item: T)
where T: Into<Item> + IdentityQuery
{
    let mut lock = (*LOST_AND_FOUND).write().await;
    log::warn!("Item '{}' added into L'n'F queue", item.id());
    lock.push(item.into());
}

/// Lost and found. Persist it all, someone might need it later…
async fn lost_and_found(world: Arc<RwLock<World>>) {
    let items_to_save = {
        let mut lock = (*LOST_AND_FOUND).write().await;
        lock.drain(..).collect::<Vec<_>>()
    };
    if items_to_save.is_empty() { return ;}
    {
        let mut w = world.write().await;
        for i in items_to_save {
            w.lost_and_found.insert(i.id().to_string(), i);
        }
    }
    save_the_whales(world.clone()).await;
}

/// ASAP save of a [HelpPage].
async fn save_help_asap() {
    let h_to_save = {
        let mut qlock = (*SAVE_HELP_ASAP).write().await;
        qlock.drain(..).collect::<Vec<_>>()
    };
    if h_to_save.is_empty() { return; }
    *(*SAVE_HELP_ASAP_STATE).write().await = HelpLibraryState::Pending;
    
    log::info!("Saving {} help document{}…", h_to_save.len(), if h_to_save.len() == 1 {""} else {"s"});
    let mut fails = vec![];
    for p in h_to_save {
        let p_id = {
            let p = p.read().await;
            p.id().to_string()
        };

        if let Err(e) = p.write().await.save().await {
            log::error!("Failed to save help document '{p_id}': {e:?}");
            fails.push(p.clone());
            continue;
        }
    }
    if !fails.is_empty() {
        (*SAVE_HELP_ASAP).write().await.extend(fails);
    }

    *(*SAVE_HELP_ASAP_STATE).write().await = HelpLibraryState::Reindex;
    log::info!("Document filing cycle complete.");
}
