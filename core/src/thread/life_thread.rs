//! Life-thread keeps the world ticking.

use std::{sync::Arc, time::Duration};

use tokio::{sync::{RwLock, mpsc}, time};

use crate::{identity::{IdentityMut, IdentityQuery}, string::Uuid, thread::{SystemSignal, librarian::ENT_BP_LIBRARY, signal::{SignalChannels, SpawnType}}, world::World};

/// Life-thread. Lives hang on in balance here!
/// 
/// Life-thread is the game's "pulse" that ticks the clocks of everything.
//TODO (It'll do) much more than that Soon™.
/// 
pub(crate) async fn life_thread((outgoing, mut incoming): (SignalChannels, mpsc::Receiver<SystemSignal>), world: Arc<RwLock<World>>) {
    let mut tick_interval = time::interval(Duration::from_millis(10));// 100Hz
    let mut tick = 0;
    log::info!("Life thread firing up…");
    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                {
                    let mut w = world.write().await;
                    w.tick().await;
                }
                tick += 1;

                #[cfg(all(debug_assertions,feature = "stresstest"))]{
                    log::trace!("tick {tick} done");
                }
            }

            Some(sig) = incoming.recv() => match sig {
                SystemSignal::Shutdown => break,
                SystemSignal::Spawn {what, room_id} => spawn_something(what, &room_id, world.clone()).await,
                _ => {}
            }
        }
    }

    log::info!("Bye now!");
}

/// Spawn a [Mob] or [Item] at given [Room] (by ID).
/// 
/// # Args
/// - `what` to spawn.
/// - `where` to spawn ([Room] ID).
async fn spawn_something(what: SpawnType, r#where: &str, world: Arc<RwLock<World>>) {
    match what {
        SpawnType::Mob { id } => {
            if let Some(mut mob) = (*ENT_BP_LIBRARY).read().await.get(&id) {
                *(mob.id_mut()) = mob.id().re_uuid();
                if let Some(r_arc) = world.read().await.rooms.get(r#where) {
                    r_arc.write().await.entities.insert(mob.id().into(), Arc::new(RwLock::new(mob)));
                } else {
                    log::error!("Ayy! We don't have room '{}' to spawn '{}' at!", r#where, mob.id());
                }
            } else {
                log::warn!("There's no record of '{id}' in the entity catalogue…");
            }
        }

        _ => ()
    }
}
