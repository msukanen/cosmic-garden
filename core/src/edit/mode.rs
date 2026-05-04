//! Editor modes.

use crate::{identity::{IdentityQuery, uniq::StrUuid}, player::Player, string::styling::dirty_mark};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Redit { dirty: bool },
    Hedit { dirty: bool },
    Iedit { dirty: bool },
    Medit  { dirty: bool },
}

impl EditorMode {
    pub fn prompt(&self, player: &Player) -> String {
        match self {
            Self::Hedit { dirty } => format!("<c blue>[<c cyan>HEDIT</c>@<c green> {} ({})</c>]</c>{}: ",
                if let Some(page) = &player.hedit_buffer { page.id().show_uuid(player.config.show_id) } else {"***"},
                if let Some(page) = &player.hedit_buffer { page.title() } else {"***"},
                dirty_mark(*dirty)
            ),
            Self::Iedit { dirty } => format!("<c blue>[<c cyan>IEDIT</c>@<c green> {}</c>]</c>{}: ",
                if let Some(item) = &player.iedit_buffer { item.id().show_uuid(player.config.show_id) } else {"***"},
                dirty_mark(*dirty)
            ),
            Self::Redit { dirty } => format!("<c blue>[<c cyan>REDIT</c>@<c green> {} ({})</c>]</c>{}: ",
                if let Some(room) = &player.redit_buffer { room.id() } else {"***"},
                if let Some(room) = &player.redit_buffer { room.title() } else {"***"},
                dirty_mark(*dirty)
            ),
            Self::Medit {dirty } => format!("<c blue>[<c cyan>MEDIT</c>@<c green> X (Y)</c>]</c>{}: ",
                dirty_mark(*dirty)
            )
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::Hedit { dirty } |
            Self::Iedit { dirty } |
            Self::Medit { dirty }  |
            Self::Redit { dirty } => *dirty
        }
    }

    pub fn set_dirty(&mut self, state: bool) {
        match self {
            Self::Hedit { dirty } |
            Self::Iedit { dirty } |
            Self::Medit { dirty }  |
            Self::Redit { dirty } => *dirty = state,
        }
    }
}
