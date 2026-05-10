//! Environment – temperature, etc.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ThermalType {
    DeepFreeze,
    Frozen,
    VeryCold,
    Cold,
    Chilly,
    Cool,
    Normal,
    Warm,
    Tropical,
    Hot,
    VeryHot,
    Infernal,
}

impl Display for ThermalType {
    /// Generate sort of ambiguous thermal "term"…
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Chilly => "chilly",
            Self::Cold => "cold",
            Self::Cool => "cool",
            Self::DeepFreeze => "deep frozen",
            Self::Frozen => "frozen",
            Self::Hot => "hot",
            Self::Infernal => "infernal",
            Self::Normal => "temperate",
            Self::Tropical => "tropical",
            Self::VeryCold => "very cold",
            Self::VeryHot => "very hot",
            Self::Warm => "warm"
        })
    }
}
