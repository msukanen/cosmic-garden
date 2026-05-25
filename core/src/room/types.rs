//! Room types…

use serde::{Deserialize, Serialize};

/// Subtypes for some [RoomType]s.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum RoomSubtype {
    /// Nothing special here to see…
    Normal,
    Desert,
    Underwater
}

/// General [Room] types.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum RoomType {
    /// `Normal` [Room] is anything from simple indoors to open plains.
    Normal,
    // Forest etc. issue some visibility factors…
    Forest, DenseForest,
    Jungle, DenseJungle,
    // Arctic generally are treated as at least `cool` or colder.
    Arctic { subtype: RoomSubtype }, DeepArctic { subtype: RoomSubtype },
    Desert,
    Tropical { subtype: RoomSubtype },
}

impl Default for RoomType {
    fn default() -> Self {
        Self::Normal
    }
}
