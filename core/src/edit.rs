//! Editor modes for those who need them.

use crate::{identity::IdentityQuery, player::Player, string::{StrUuid, styling::dirty_mark}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Room { dirty: bool },
    Help { dirty: bool },
    Item { dirty: bool },
    Mob  { dirty: bool },
}

impl EditorMode {
    pub fn prompt(&self, player: &Player) -> String {
        match self {
            Self::Help { dirty } => format!("<c blue>[<c cyan>HEDIT</c>@<c green> {} ({})</c>]</c>{}: ",
                if let Some(page) = &player.hedit_buffer { page.id().show_uuid(player.config.show_id) } else {"***"},
                if let Some(page) = &player.hedit_buffer { page.title() } else {"***"},
                dirty_mark(*dirty)
            ),
            Self::Item { dirty } => format!("<c blue>[<c cyan>IEDIT</c>@<c green> {}</c>]</c>{}: ",
                if let Some(item) = &player.iedit_buffer { item.id().show_uuid(player.config.show_id) } else {"***"},
                dirty_mark(*dirty)
            ),
            Self::Room { dirty } => format!("<c blue>[<c cyan>REDIT</c>@<c green> {} ({})</c>]</c>{}: ",
                if let Some(room) = &player.redit_buffer { room.id() } else {"***"},
                if let Some(room) = &player.redit_buffer { room.title() } else {"***"},
                dirty_mark(*dirty)
            ),
            Self::Mob {dirty } => format!("<c blue>[<c cyan>MEDIT</c>@<c green> X (Y)</c>]</c>{}: ",
                dirty_mark(*dirty)
            )
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::Help { dirty } |
            Self::Item { dirty } |
            Self::Mob { dirty }  |
            Self::Room { dirty } => *dirty
        }
    }

    pub fn set_dirty(&mut self, state: bool) {
        match self {
            Self::Help { dirty } |
            Self::Item { dirty } |
            Self::Mob { dirty }  |
            Self::Room { dirty } => *dirty = state,
        }
    }
}
