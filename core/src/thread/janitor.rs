//! Disk I/O threading…

use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{sync::RwLock, time};

use crate::{Cli, error::CgError, identity::{IdentityQuery, MachineIdentity}, item::Item, player::PlayerArc, room::{Room, RoomArc}, thread::{SystemSignal, signal::{SigReceiver, SignalSenderChannels}}, world::WorldArc};

lazy_static! {
    pub static ref ROOMS_TO_SAVE: Arc<RwLock<Vec<RoomArc>>> = Arc::new(RwLock::new(Vec::new()));
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
    world: WorldArc,
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
    let mut timing_out = false;

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
                SystemSignal::TimedShutdown { mut delay } => {
                    if timing_out {
                        log::warn!("Multiple shutdown commands issued at the same time.");
                        continue;
                    }
                    timing_out = true;
                    let tsd_out = out.clone();
                    tokio::spawn(async move {
                        while delay > 0 {
                            tsd_out.broadcast.send(crate::io::Broadcast::TimedShutdown { seconds: delay }).ok();
                            delay /= 2;
                            tokio::time::sleep(Duration::from_secs(delay as u64)).await;
                        }
                        tsd_out.broadcast.send(crate::io::Broadcast::Shutdown).ok();
                        tsd_out.life.send(crate::thread::SystemSignal::AbortAllBattle).ok();
                        tsd_out.shutdown().await;
                    });
                }

                SystemSignal::SaveWorld => {save_the_whales(world.clone(), true).await;}
                SystemSignal::LostAndFound => lost_and_found(world.clone()).await,
                SystemSignal::PlayerNeedsSaving(lock) => {
                    let p_id = lock.read().await.id().to_string();
                    save_player_now(lock, &p_id).await;
                    #[cfg(test)]{log::trace!("Janitor books '{p_id}' as \"processed\".");}
                }
                SystemSignal::SaveRoom { arc } => {tokio::spawn(async move {
                    log::trace!("Saving '{}'…", arc.read().await.id());
                    arc.write().await.save().await.ok();
                });}
                SystemSignal::ReloadRoom { arc } => {
                    #[cfg(test)]{ log::debug!("Oh cool, a request to reload a room received in mail…");}
                    let world = world.clone();
                    tokio::spawn(async move {
                        if let Err(e) = reload_room(world, arc).await {
                            log::error!("Sheesh, the room file is in fire?! {e:?}");
                        };
                    });
                }

                _ => ()
            }
        }
    }

    // re-send shutdown just in case...
    out.shutdown().await;
    // Ok, lights out …!
    lost_and_found(world.clone()).await; // drain LOST_AND_FOUND queue just in case…
    save_the_whales(world.clone(), true).await;// save the whales!
    room_save().await;// …and the rooms
    autosave_queue(world.clone()).await;// well, and players.

    // notify main thread that we've closed the shop.
    let _ = done_tx.send(());
    log::info!("Janitor checking out.");
}

#[cfg(test)]
// stub for #[cfg(test)].
async fn save_player_now(_: PlayerArc, _: &str) {
    log::debug!("[STUB] save_player_now() disabled in #[cfg(test)].");
}
#[cfg(not(test))]
/// Save player ASAP.
async fn save_player_now(plr: PlayerArc, p_id: &str) {
    let mut p = plr.read().await.clone();
    let act_w = p.actions_taken;
    if let Err(e) = p.save().await {
        log::error!("Failed to save player '{p_id}': {e:?}")
    }
    let mut p = plr.write().await;
    p.actions_taken = p.actions_taken.saturating_sub(act_w);
    log::trace!("Saved '{p_id}'…");
}

/// Auto-save cycle.
async fn autosave_queue(world: WorldArc) {
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
        let p_id = p.read().await.id().to_string();
        save_player_now(p.clone(), &p_id).await;
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
async fn save_the_whales(world: WorldArc, force_save: bool) -> bool {
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
        let mut qlock = ROOMS_TO_SAVE.write().await;
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
        ROOMS_TO_SAVE.write().await.extend(rooms_to_requeue);
    }
}

/// Add an [Item] into Lost'n'Found queue.
pub async fn add_item_to_lnf<T>(item: T)
where T: Into<Item> + IdentityQuery
{
    let mut lock = LOST_AND_FOUND.write().await;
    log::warn!("Item '{}' added into L'n'F queue", item.id());
    lock.push(item.into());
}

/// Lost and found. Persist it all, someone might need it later…
async fn lost_and_found(world: WorldArc) {
    let items_to_save = {
        let mut lock = LOST_AND_FOUND.write().await;
        lock.drain(..).collect::<Vec<_>>()
    };
    if items_to_save.is_empty() { return ;}
    {
        let mut w = world.write().await;
        for i in items_to_save {
            w.lost_and_found.insert(i.id().as_m_id(), i);
        }
    }

    // Force world save so that current L'n'F is stored along the world file.
    // This is separately called during wind down, too, but…
    save_the_whales(world.clone(), true).await;
}

/// Reload a room from disk.
async fn reload_room(world: WorldArc, arc: RoomArc) -> Result<(), CgError> {
    let r = {
        let rw = arc.read().await;
        Room::load_sync(rw.id())?
    };
    arc.write().await.scavenge(r, &world).await;
    Ok(())
}

#[cfg(test)]
mod janitor_tests {
    use std::io::Cursor;

    use crate::{cmd::{look::LookCommand, reload::ReloadCommand}, get_operational_mock_librarian, stabilize_threads, util::access::Access, world::mock_world::get_operational_mock_world};

    #[tokio::test]
    async fn janitor_reload_room() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state, p),d) = get_operational_mock_world().await;
        get_operational_mock_janitor!(c,w,d.0);
        get_operational_mock_librarian!(c,w);
        stabilize_threads!();
        let c = c.out;
        state = ctx!(sup state, ReloadCommand,"",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Player { event_host: false, builder: true };
        state = ctx!(sup state, ReloadCommand,"",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let Some(rw) = w.read().await.get_room_by_id("r-1").clone() else {
            panic!("Where'd the room go?!");
        };
        _ = rw.read().await.save().await;
        rw.write().await.desc = "Very, very roomy".into();
        state = ctx!(sup state, LookCommand,"",s,c,w,|out:&str| out.contains("Very, very"));
        state = ctx!(state, ReloadCommand, "",s,c,w,|out:&str| out.contains("'reload'"));
        state = ctx!(state, ReloadCommand, "googolplex",s,c,w,|out:&str| out.contains("no such place"));
        state = ctx!(state, ReloadCommand, "here",s,c,w,|out:&str| out.contains("Requested"));
        stabilize_threads!(10);//let's wait reload to actually happen…
        _ = ctx!(sup state, LookCommand,"",s,c,w,|out:&str| !out.contains("Very, very"));
        c.shutdown().await;
        _ = d.1.await;
    }
}
