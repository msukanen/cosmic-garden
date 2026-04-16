//! Errors ad hominis…

use crate::{identity::IdError, item::container::StorageError, password::PasswordError, room::RoomError};

/// Cosmic Garden error type - a thin wrapper around more specific ones.
// Rarely of any use for other but sanitizing [Debug] output.
#[derive(Debug, thiserror::Error)]
pub enum CgError {
    #[error("Matter-Collapse: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O gone bonkers: {0}")]
    Io(#[from] std::io::Error),

    #[error("Identity Crisis: {0}")]
    Id(#[from] IdError),

    #[error("Oh dear…: {0}")]
    Storage(#[from] StorageError),

    #[error("Need more argon… {0}")]
    Password(#[from] PasswordError),

    #[error("Room fail! {0}")]
    Room(#[from] RoomError),

    #[error("Document-Illegible: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("Document-Fire: {0}")]
    TomlSer(#[from] toml::ser::Error)
}
