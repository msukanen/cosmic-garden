use crate::{combat::Battler, player::PlayerArc, room::RoomArc};

/// Various broadcast types.
#[derive(Clone)]
pub enum Broadcast {
    /// 'say' something.
    Say {
        room: RoomArc,
        message: String,
        from: PlayerArc,
    },

    /// 'who' is moving 'from' 'to'.
    Movement {
        from: RoomArc,
        to: RoomArc,
        who: PlayerArc,
    },

    /// 'who' logs out.
    Logout {
        from: RoomArc,
        who: String,
    },

    /// 'message' ('from'…) to all in given 'room(s)'.
    System {
        rooms: Vec<RoomArc>,
        message: String,
        from: Option<PlayerArc>,
    },

    Message {
        to: PlayerArc,
        message: String,
    },

    /// 'who' is moving 'from' 'to' with message for a) 'to' b) 'from' c) self.
    BiSignal {
        to: RoomArc,
        from: RoomArc,
        who: PlayerArc,
        message_to: String,
        message_from: String,
        message_who: String,
    },

    MessageInRoom2 {
        room: RoomArc,
        actor: PlayerArc,
        message_actor: String,
        message_other: String,
    },

    MessageInRoom {
        room: RoomArc,
        message: String,
    },

    BattleMessage3 {
        room: RoomArc,
        atk: Battler,
        vct: Battler,
        message_atk: String,
        message_vct: String,
        message_other: String,
    },

    Force {
        command: String,
        who: ForceTarget,
        by: Option<PlayerArc>,
        silent: bool,
        delivery: Option<String>,
    },

    Shutdown,
}

#[derive(Debug, Clone)]
pub enum ForceTarget {
    Room { id: RoomArc },
    Player { id: PlayerArc },
    All,
}
