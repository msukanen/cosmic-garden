//! Disk I/O threading…

use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{sync::RwLock, time};

use crate::{Cli, identity::IdentityQuery, item::Item, player::Player, room::Room, thread::{SystemSignal, librarian::{BP_LIBRARY, HELP_LIBRARY}, signal::{SigReceiver, SignalSenderChannels}}, world::World};

lazy_static! {
    pub static ref ROOMS_TO_SAVE: Arc<RwLock<Vec<Arc<RwLock<Room>>>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref LOST_AND_FOUND: Arc<RwLock<Vec<Item>>> = Arc::new(RwLock::new(Vec::new()));
}
pub const SAVE_ASAP_THRESHOLD: usize = 100;

#[cfg(test)]
#[macro_export]
macro_rules! get_operational_mock_janitor {
    ($ch:ident, $w:ident, $done_tx:expr) => {
        tokio::spawn( crate::thread::janitor(($ch.out.clone(), $ch.recv.janitor), $w.clone(), None, $done_tx))
    };
}

/// Disk I/O thread thing.
/// 
/// `janitor` handles all sorts of "janitorial" tasks from
/// autosaves and logouts to keeping the live world and disk
/// in (relative) sync.
/// 
pub(crate) async fn janitor(
    (out, mut incoming): (SignalSenderChannels, SigReceiver),
    world: Arc<RwLock<World>>,
    args: Option<Cli>,
    done_tx: tokio::sync::oneshot::Sender<()>
) {
    let asqi = if let Some(args) = &args {
        args.autosave_queue_interval.unwrap_or(300)
    } else { 300 };
    let mut autosave_queue_interval = time::interval(Duration::from_secs(asqi));
    let mut world_save_interval = time::interval(Duration::from_mins(2));
    let mut room_save_interval = time::interval(Duration::from_secs(30));
    let mut lost_and_found_interval = time::interval(Duration::from_mins(2));

    log::trace!("Janitor in the house! \"Time to keep this place tidy…\"");

    loop {
        tokio::select! {
            // Auto-saving the meningfully active Players.
            _ = autosave_queue_interval.tick() => autosave_queue(world.clone()).await,
            // Save the world, especially the whales!
            _ = world_save_interval.tick() => {save_the_whales(world.clone(), false).await;},
            // Save the modified [Room]s.
            _ = room_save_interval.tick() => room_save().await,
            // Handle lost and found items…
            _ = lost_and_found_interval.tick() => lost_and_found(world.clone()).await,
            // Anything in mailbox?
            Some(sig) = incoming.recv() => match sig {
                SystemSignal::Shutdown => break,

                SystemSignal::SaveWorld => {save_the_whales(world.clone(), true).await;},
                SystemSignal::LostAndFound => lost_and_found(world.clone()).await,
                SystemSignal::ReindexLibrary => save_help_asap(&out).await,
                SystemSignal::NewBlueprintEntry => save_bp_asap(&out).await,
                SystemSignal::PlayerNeedsSaving(lock) => {
                    let p_id = lock.read().await.id().to_string();
                    save_player_now(lock, &p_id).await;
                    #[cfg(test)]{log::trace!("Janitor books '{p_id}' as \"processed\".");}
                }
                _ => ()
            }
        }
    }

    // Ok, lights out …!
    lost_and_found(world.clone()).await; // drain LOST_AND_FOUND queue just in case…
    save_the_whales(world.clone(), true).await;// save the whales!
    room_save().await;// …and the rooms
    autosave_queue(world.clone()).await;// well, and players.
    save_help_asap(&out).await;
    save_bp_asap(&out).await;

    // notify main thread that we've closed the shop.
    let _ = done_tx.send(());
    log::info!("Janitor checking out.");
}

/// Save player ASAP.
async fn save_player_now(plr: Arc<RwLock<Player>>, p_id: &str) {
    let p = plr.read().await.clone();
    let act_w = p.actions_taken;
    if let Err(e) = p.save().await {
        log::error!("Failed to save player '{p_id}': {e:?}")
    }
    let mut p = plr.write().await;
    p.actions_taken = p.actions_taken.saturating_sub(act_w);
    log::trace!("Saved '{p_id}'…");
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
        log::info!("Player autosave cycle initiated.");
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
        log::info!("Player autosave cycle complete.");
    }
}

/// World save cycle.
/// 
/// # Args
/// - `force_save` if true, the world state is stored regardless of "necessity".
///                Generally used when a [Room] is added/removed.
async fn save_the_whales(world: Arc<RwLock<World>>, force_save: bool) -> bool {
    if let Err(e) = world.read().await.save(force_save).await {
        log::error!("World save failed?! {e:?}");
        return false;
    }
    if force_save {
        log::info!("Forced world save cycle complete.");
    }
    return true;
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

    // Force world save so that current L'n'F is stored along the world file.
    save_the_whales(world.clone(), true).await;
}

/// ASAP save of [HelpPage]s.
async fn save_help_asap(out: &SignalSenderChannels) {
    if let Err(e) = (*HELP_LIBRARY).write().await.save().await {
        log::error!("Help anomaly! {e:?}");
        return ;
    }
    // Nudge the librarian, but don't wait if he's asleep…
    out.librarian.send(SystemSignal::ReindexLibrary).ok();
}

/// ASAP save of [HelpPage]s.
async fn save_bp_asap(out: &SignalSenderChannels) {
    if let Err(e) = (*BP_LIBRARY).write().await.save().await {
        log::error!("Blueprint anomaly! {e:?}");
        return ;
    }
    // Nudge the librarian, but don't wait if he's asleep…
    out.librarian.send(SystemSignal::ReindexLibrary).ok();
}
