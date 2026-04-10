//! Disk I/O threading…

use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{sync::RwLock, time};

use crate::{Cli, DATA_PATH, identity::IdentityQuery, player::Player, world::World};

lazy_static! {
    pub static ref PLAYERS_TO_LOGOUT: Arc<RwLock<Vec<Arc<RwLock<Player>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref WORLD_NEEDS_SAVING: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
}

/// Disk I/O thread thing.
pub(super) async fn io_thread(world: Arc<RwLock<World>>, args: Cli) {
    log::trace!("Firing up; DATA_PATH = '{}'", *DATA_PATH);

    let mut autosave_queue_interval = time::interval(Duration::from_secs(args.autosave_queue_interval.unwrap_or(300)));
    let mut logout_purge_interval = time::interval(Duration::from_secs(1));
    let mut world_save_interval = time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Logout handling.
            _ = logout_purge_interval.tick() => {
                let players_to_save = {
                    let mut qlock = (*PLAYERS_TO_LOGOUT).write().await;
                    qlock.drain(..).collect::<Vec<Arc<RwLock<Player>>>>()
                };

                if !players_to_save.is_empty() {
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
            }

            // Auto-saving the meningfully active Players.
            _ = autosave_queue_interval.tick() => {
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

            _ = world_save_interval.tick() => {
                let ws_req = {
                    let w = (*WORLD_NEEDS_SAVING).write().await;
                    *w
                };
                if ws_req {
                    *((*WORLD_NEEDS_SAVING).write().await) = false;
                    if let Err(e) = world.write().await.save().await {
                        log::error!("World save failed?! {e:?}");
                        *((*WORLD_NEEDS_SAVING).write().await) = true
                    } else {
                        log::info!("World save cycle complete.");
                    }
                }
            }
        }
    }
}
