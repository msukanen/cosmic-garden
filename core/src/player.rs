//! Player stuff!

use std::sync::Arc;

use cosmic_garden_pm::IdentityMut;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::{error::Error, io::{ClientState, SAVE_PATH}, string::UNNAMED};

/// A player's character contained here…
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut)]
pub struct Player {
    /// ID of owner of this specific [Player] character.
    pub(super) owner_id: String,
    id: String,
    #[identity(title)]
    pub(super) name: String,
}

impl Player {
    pub fn owner_id<'a>(&'a self) -> &'a str { &self.owner_id }

    pub async fn load(owner_id: &str, id: &str) -> Result<Arc<RwLock<Self>>, Error> {
        let player: Self = serde_json::from_str(
            &fs::read_to_string(&format!("{}/{}-{}.player", SAVE_PATH.display(), owner_id, id)).await?
        )?;
        Ok(Arc::new(RwLock::new(player)))
    }

    pub async fn save(&self) -> Result<(), Error> {
        let path = format!("{}/{}-{}.player", SAVE_PATH.display(), self.owner_id, self.id);
        log::debug!("Storing {path}");
        fs::write(path, serde_json::to_string_pretty(self)?).await?;
        Ok(())
    }

    pub fn prompt(&self, state: &ClientState) -> Option<String> {
        match state {
            ClientState::Playing { .. } => format!("[HP X/Y, MP X/Y, SN X/Y]#> ").into(),
            ClientState::Editing { mode, .. } => mode.prompt(&self).into(),
            _ => None// all other states are dealt by I/O machinery directly.
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            owner_id: UNNAMED.into(),
            id: "".into(),
            name: "".into(),
        }
    }
}
