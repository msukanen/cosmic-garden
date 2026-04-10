use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{player::Player, room::Room};

/// Various broadcast types.
#[derive(Debug, Clone)]
pub enum Broadcast {
    Say {
        room: Arc<RwLock<Room>>,
        message: String,
        from: Arc<RwLock<Player>>,
    },

    Arrival {
        from: Arc<RwLock<Room>>,
        to: Arc<RwLock<Room>>,
        who: Arc<RwLock<Player>>,
    }
}
