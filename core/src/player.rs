//! Player stuff!

use std::sync::{Arc, Weak};

use cosmic_garden_pm::IdentityMut;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::{error::Error, identity::IdentityQuery, io::{ClientState, SAVE_PATH}, room::Room, string::UNNAMED, world::World};

/// A player's character contained here…
#[derive(Debug, Clone, Deserialize, Serialize, IdentityMut)]
pub struct Player {
    /// ID of owner of this specific [Player] character.
    pub(super) owner_id: String,
    id: String,
    #[identity(title)]
    pub(super) name: String,
    #[serde(default, skip)]
    pub actions_taken: usize,
    #[serde(default = "player_location_void")]
    location_id: String,
    #[serde(skip)]
    pub location: Weak<RwLock<Room>>,
}

fn player_location_void() -> String {
    UNNAMED.into()
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

    pub async fn place(player: Arc<RwLock<Player>>, world: Arc<RwLock<World>>, target_id: &str) -> Result<(), Error> {
        if let Some(target_arc) = world.read().await.rooms.get(target_id) {
            Player::place_direct(player.clone(), world.clone(), target_arc.clone()).await?
        }
        Ok(())
    }

    pub async fn place_direct(player: Arc<RwLock<Player>>, world: Arc<RwLock<World>>, target_arc: Arc<RwLock<Room>>) -> Result<(), Error> {
        let mut tgt_lock = target_arc.write().await;
        let mut p_lock = player.write().await;
        tgt_lock.who.insert(p_lock.id().into(), Arc::downgrade(&player));
        if let Some(origin) = p_lock.location.upgrade() {
            let mut origin_lock = origin.write().await;
            origin_lock.who.remove(p_lock.id());
        }
        p_lock.location_id = tgt_lock.id().into();
        p_lock.location = Arc::downgrade(&target_arc);
        log::debug!("Placing player at '{}'", tgt_lock.id());
        Ok(())
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            owner_id: UNNAMED.into(),
            id: "".into(),
            name: "".into(),
            actions_taken: 0,
            location_id: player_location_void(),
            location: Weak::new(),
        }
    }
}
