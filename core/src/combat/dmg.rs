//! Combat dmg types and stuff.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::environ::ThermalType;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum DamageType {
    Corrode,
    Crush,
    Cut,
    Impale,
    Thermal(ThermalType),
}

impl Display for DamageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Corrode => "corrode",
            Self::Crush => "crush",
            Self::Cut => "cut",
            Self::Impale => "impale",
            Self::Thermal(t) => return write!(f, "thermal ({})", match t {
                ThermalType::Cold       |
                ThermalType::DeepFreeze |
                ThermalType::Frozen     => "cold",
                ThermalType::Hot        |
                ThermalType::VeryHot    |
                ThermalType::Infernal   => "heat",
                err => {
                    log::error!("DamageType logic error: DamageType::Thermal({err})");
                    "…flu…" // should be unreachable, but…
                }
            })
        })
    }
}
