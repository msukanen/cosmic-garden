//! Quest objective stuff.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Objective {
    Delivery,
    Fetch,
    Kill,
}
