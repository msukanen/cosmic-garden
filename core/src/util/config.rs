//! Configurable things for [Player]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub show_self_in_room: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_self_in_room: true,
        }
    }
}
