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

    Movement {
        from: Arc<RwLock<Room>>,
        to: Arc<RwLock<Room>>,
        who: Arc<RwLock<Player>>,
    },

    Logout {
        from: Arc<RwLock<Room>>,
        who: String,
    },

    System {
        rooms: Vec<Arc<RwLock<Room>>>,
        message: String,
        sender: Option<Arc<RwLock<Player>>>,
    },

    BiSignal {
        to: Arc<RwLock<Room>>,
        from: Arc<RwLock<Room>>,
        who: Arc<RwLock<Player>>,
        message_to: String,
        message_from: String,
        message_who: String,
    },

    SystemInRoom {
        room: Arc<RwLock<Room>>,
        actor: Arc<RwLock<Player>>,
        message_actor: String,
        message_other: String,
    },

    Force {
        command: String,
        who: ForceTarget,
        by: Arc<RwLock<Player>>,
        delivery: Option<String>,
    },

    Shutdown,
}

#[derive(Debug, Clone)]
pub enum ForceTarget {
    Room { id: Arc<RwLock<Room>> },
    Player { id: Arc<RwLock<Player>> },
    All,
}
