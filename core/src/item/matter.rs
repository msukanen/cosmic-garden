//! Matter basics…

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum MatterState {
    Liquid,
    Solid,
    Gaseous,
    Plasma,
}

impl MatterState {
    pub fn delivery_method(&self) -> &'static str {
        match self {
            Self::Solid => "eat",
            Self::Liquid => "drink",
            Self::Plasma => "absorb",
            Self::Gaseous => "inhale",
        }
    }
}
