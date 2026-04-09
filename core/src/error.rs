//! Errors ad hominis…

use crate::{identity::IdError, item::container::StorageError, password::PasswordError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
}
