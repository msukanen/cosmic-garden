//! Editor modes for those who need them.

use crate::player::Player;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Room,
    Help,
}

impl EditorMode {
    pub fn prompt(&self, player: &Player) -> String {
        match self {
            Self::Help => "[HEDIT ()]: ",
            Self::Room => "[REDIT ()]: ",
        }.into()
    }
}