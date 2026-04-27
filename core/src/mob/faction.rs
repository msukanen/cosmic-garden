//! Mobs have 'factions' (Players too…).

use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// Some basic/preliminary factions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum EntityFaction {
    Player { pvp: bool },
    Guard,

    Friendly,
    Neutral,
    Hostile,
}

pub trait Factioned {
    fn faction(&self) -> EntityFaction;
}

pub trait FactionMut {
    fn faction_mut(&mut self) -> &mut EntityFaction;
}

impl Display for EntityFaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Friendly => write!(f, "friendly"),
            Self::Neutral => write!(f, "neutral"),
            Self::Hostile => write!(f, "hostile"),
            Self::Guard => write!(f, "guard"),
            Self::Player { pvp } => write!(f, "player{}", if *pvp {" (PvP)"} else {""}),
        }
    }
}