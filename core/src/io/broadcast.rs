use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{player::Player, room::Room};

/// Various broadcast types.
#[derive(Debug, Clone)]
pub enum Broadcast {
    /// 'say' something.
    Say {
        room: Arc<RwLock<Room>>,
        message: String,
        from: Arc<RwLock<Player>>,
    },

    /// 'who' is moving 'from' 'to'.
    Movement {
        from: Arc<RwLock<Room>>,
        to: Arc<RwLock<Room>>,
        who: Arc<RwLock<Player>>,
    },

    /// 'who' logs out.
    Logout {
        from: Arc<RwLock<Room>>,
        who: String,
    },

    /// 'message' ('from'…) to all in given 'room(s)'.
    System {
        rooms: Vec<Arc<RwLock<Room>>>,
        message: String,
        from: Option<Arc<RwLock<Player>>>,
    },

    Message {
        to: Arc<RwLock<Player>>,
        message: String,
    },

    /// 'who' is moving 'from' 'to' with message for a) 'to' b) 'from' c) self.
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

    SystemInRoomAt {
        room: Arc<RwLock<Room>>,
        atk: Arc<RwLock<Player>>,
        vct: Arc<RwLock<Player>>,
        message_atk: String,
        message_vct: String,
        message_other: String,
    },

    Force {
        command: String,
        who: ForceTarget,
        by: Option<Arc<RwLock<Player>>>,
        silent: bool,
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
