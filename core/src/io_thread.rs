//! Disk I/O threading…

use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{sync::RwLock, time};

use crate::{Cli, DATA_PATH, identity::IdentityQuery, item::Item, player::Player, room::Room, world::World};

lazy_static! {
    pub static ref PLAYERS_TO_LOGOUT: Arc<RwLock<Vec<Arc<RwLock<Player>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref WORLD_NEEDS_SAVING: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    pub static ref ROOMS_TO_SAVE: Arc<RwLock<Vec<Arc<RwLock<Room>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref SAVE_ASAP: Arc<RwLock<Vec<Arc<RwLock<Player>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref LOST_AND_FOUND: Arc<RwLock<Vec<Item>>> = Arc::new(RwLock::new(Vec::new()));
}
pub const SAVE_ASAP_THRESHOLD: usize = 100;

/// Disk I/O thread thing.
pub(super) async fn io_thread(world: Arc<RwLock<World>>, args: Cli) {
    log::trace!("Firing up; DATA_PATH = '{}'", *DATA_PATH);

    let mut autosave_queue_interval = time::interval(Duration::from_secs(args.autosave_queue_interval.unwrap_or(300)));
    let mut logout_purge_interval = time::interval(Duration::from_secs(1));
    let mut world_save_interval = time::interval(Duration::from_secs(30));
    let mut room_save_interval = time::interval(Duration::from_secs(30));
    let mut save_asap_interval = time::interval(Duration::from_secs(2));
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
        }
    }
}

/// Purge logged out players.
async fn logout_purge() {
    let players_to_save = {
        let mut qlock = (*PLAYERS_TO_LOGOUT).write().await;
        qlock.drain(..).collect::<Vec<_>>()
    };
    if players_to_save.is_empty() { return ; }
    
    log::info!("Saving {} disconnected player{}…", players_to_save.len(), if players_to_save.len() == 1 {""} else {"s"});
    for p in players_to_save {
        let p_id = {
            let p = p.read().await;
            p.id().to_string()
        };
        if let Err(e) = p.write().await.save().await {
            log::error!("Failed to save player '{p_id}': {e:?}")
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
    let ws_req = {
        let mut w = (*WORLD_NEEDS_SAVING).write().await;
        let val = *w;
        *w = false;
        val
    };
    if !ws_req { return ; }

    if let Err(e) = world.read().await.save().await {
        log::error!("World save failed?! {e:?}");
        *((*WORLD_NEEDS_SAVING).write().await) = true;
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
pub async fn add_item_to_lnf(item: Item) {
    let mut lock = (*LOST_AND_FOUND).write().await;
    log::warn!("Item '{}' added into L'n'F queue", item.id());
    lock.push(item);
}

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
