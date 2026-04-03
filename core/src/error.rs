//! Errors ad hominis…

use crate::identity::IdError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Matter-Collapse: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O gone bonkers: {0}")]
    Io(#[from] std::io::Error),

    #[error("Identity Crisis: {0}")]
    Id(#[from] IdError),
}
