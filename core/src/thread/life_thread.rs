//! Life-thread keeps the world ticking.

use std::{sync::Arc, time::Duration};

use tokio::{sync::RwLock, time};

use crate::world::World;

/// Life-thread. Lives hang on in balance here!
/// 
/// Life-thread is the game's "pulse" that ticks the clocks of everything.
//TODO (It'll do) much more than that Soon™.
/// 
pub(crate) async fn life_thread(world: Arc<RwLock<World>>) {
    let mut tick_interval = time::interval(Duration::from_millis(100));
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
        }
    }
}
