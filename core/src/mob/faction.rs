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

pub enum EntityFactionError {
    VoidFaction,
    NotFaction(String),
    CannotSetPlayerPVP,
}

impl Display for EntityFactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CannotSetPlayerPVP => write!(f, "Only player can set themselves into Hardcore/PvP mode."),
            Self::NotFaction(x) => write!(f, "'{x}' isn't a recognized \"faction\"."),
            Self::VoidFaction => write!(f, "An empty string doesn't represent any faction whatsoever…"),
        }
    }
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

impl TryFrom<&str> for EntityFaction {
    type Error = EntityFactionError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() { return Err(EntityFactionError::VoidFaction) }
        Ok(match value.chars().nth(0).unwrap() {
            'f'|'F' => Self::Friendly,
            'n'|'N' => Self::Neutral,
            'g'|'G' => Self::Guard,
            'h'|'H'|
            'a'|'A' => Self::Hostile,
            'p' => return Err(EntityFactionError::CannotSetPlayerPVP),
            _ => return Err(EntityFactionError::NotFaction(value.into()))
        })
    }
}
