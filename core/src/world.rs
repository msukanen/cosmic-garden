//! When worlds collide…
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};

use crate::{error::Error, io::DATA_PATH, player::Player, string::prompt::PromptType};

/// The world!
#[derive(Debug, Deserialize, Serialize)]
pub struct World {
    /// World's printable name.
    pub name: String,
    /// Location of the .world file itself.
    #[serde(skip)]
    path: PathBuf,
    /// Port# the world listens on.
    pub port: u16,

    /// Optional greeting message override.
    #[serde(default)]
    pub greeting: Option<String>,
    /// Optional prompt overrides.
    #[serde(default)]
    pub fixed_prompts: HashMap<PromptType, String>,

    /// Players sorted by their direct socket address.
    #[serde(skip, default)]
    pub players_by_sockaddr: HashMap<SocketAddr, Arc<RwLock<Player>>>,
    /// Players sorted by user's login ID (not their [Player] character's ID).
    #[serde(skip, default)]
    pub players_by_id: HashMap<String, Arc<RwLock<Player>>>,
}

impl World {
    pub async fn load_or_bootstrap(file_stem: &str) -> Result<Self, Error> {
        let path = PathBuf::from(format!("{}/{file_stem}.world", *DATA_PATH));
        let mut world: World = serde_json::from_str( &fs::read_to_string(&path).await? )?;
        world.path = path;
        Ok(world)
    }
}
