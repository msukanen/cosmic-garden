//! Editor modes for those who need them.

use crate::{identity::IdentityQuery, player::Player};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Room { dirty: bool },
    Help { dirty: bool },
    Item { dirty: bool },
}

impl EditorMode {
    pub fn prompt(&self, player: &Player) -> String {
        match self {
            Self::Help{dirty} => "[HEDIT ()]: ".to_string(),
            Self::Item{dirty} => "[IEDIT ()]: ".to_string(),
            Self::Room{dirty} => format!("<c blue>[<c cyan>REDIT</c>@<c green>{} ({})</c>]</c>{}: ",
                if let Some(room) = &player.redit_buffer { room.id() } else {"***"},
                if let Some(room) = &player.redit_buffer { room.title() } else {"***"},
                if *dirty {"<c red>^<c yellow>*</c></c>"} else {""}
            ),
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::Help { dirty } |
            Self::Item { dirty } |
            Self::Room { dirty } => *dirty
        }
    }

    pub fn set_dirty(&mut self, state: bool) {
        match self {
            Self::Help { dirty } |
            Self::Item { dirty } |
            Self::Room { dirty } => *dirty = state,
        }
    }
}
