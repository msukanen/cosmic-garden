//! User access levels.

use serde::{Deserialize, Serialize};

/// [Player] access rights.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Access {
    Admin,
    Builder,
    Player {
        event_host: bool,
        builder: bool,
    }
}

impl Default for Access {
    fn default() -> Self {
        Self::Player { event_host: false, builder: false }
    }
}

/// Accessor for all things [Access].
pub trait Accessor {
    /// Do they have admin rights?
    fn is_admin(&self) -> bool;
    /// Do they have builder rights?
    fn is_builder(&self) -> bool;
    /// Are they a concrete builder instead of an assisting player who's nominated as such?
    fn is_true_builder(&self) -> bool;
    /// Are they an event host?
    fn is_event_host(&self) -> bool;
}

impl Accessor for Access {
    fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }

    fn is_builder(&self) -> bool {
        match self {
            Self::Player { builder: false,.. } => false,
            _ => true
        }
    }

    fn is_true_builder(&self) -> bool {
        matches!(self, Self::Admin | Self::Builder)
    }

    fn is_event_host(&self) -> bool {
        match self {
            Self::Player { event_host: true,.. } |
            Self::Admin => true,
            _ => false
        }
    }
}

#[macro_export]
macro_rules! player_or_bust {
    ($ctx:ident) => {{
        let Some(plr) = $ctx.get_player_arc() else {
            crate::tell_user_unk!($ctx.writer);
            log::error!("Where'd the Player go?!");
            return;
        };
        plr
    }};
}

#[macro_export]
macro_rules! validate_access {
    ($ctx:ident, $type:ident) => {
        {
            let Some(plr) = $ctx.get_player_arc() else {
                crate::tell_user_unk!($ctx.writer);
                return;
            };
            let p = plr.read().await;
            use crate::util::access::Accessor;
            paste::paste! {
            if !p.access.[<is_ $type>]() {
                crate::tell_user_unk!($ctx.writer);
                return;
            }
            }
            drop(p);
            plr
        }
    };
}

#[macro_export]
macro_rules! roomloc_or_bust {
    ($plr:ident) => {
        {
            let p = $plr.read().await;
            let Some(p_loc) = p.location.upgrade() else {
                return;
            };
            p_loc
        }
    };
}