//! Life-thread keeps the world ticking.

use std::{sync::Arc, time::Duration};

use tokio::{sync::{RwLock, mpsc}, time};

use crate::{thread::{SystemSignal, signal::SignalChannels}, world::World};

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
                _ => {}
            }
        }
    }

    log::info!("Bye now!");
}
