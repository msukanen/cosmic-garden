//! Matter basics…

use std::fmt::Display;

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

impl Display for MatterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Gaseous => "gaseous",
            Self::Liquid => "liquid",
            Self::Plasma => "plasma",
            Self::Solid => "solid",
        })
    }
}

pub trait Matter {
    fn matter_state(&self) -> MatterState;
}
