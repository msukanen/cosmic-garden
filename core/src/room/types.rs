//! Room types…

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum RoomSubtype {
    Normal,
    Desert,
    Underwater
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum RoomType {
    Normal,
    Forest, DenseForest,
    Jungle, DenseJungle,
    Arctic { subtype: RoomSubtype }, DeepArctic { subtype: RoomSubtype },
    Desert,
    Tropical { subtype: RoomSubtype }
}

impl Default for RoomType {
    fn default() -> Self {
        Self::Normal
    }
}
