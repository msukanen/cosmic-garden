//! Mobs have 'factions' (Players too…).

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Demeanor {
    Friendly,
    Neutral,
    Hostile,
}

impl Display for Demeanor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Friendly => "<c green> (friendly)</c>",
            Self::Neutral  => "<c gray> (neutral)</c>",
            Self::Hostile  => "<c red> [HOSTILE!]</c>"
        })
    }
}

#[derive(Debug)]
pub enum DemeanorError {
    NotDemeanor(char)
}

impl TryFrom<&str> for Demeanor {
    type Error = DemeanorError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.chars().nth(0).unwrap_or_else(||'n'))
    }
}

impl TryFrom<char> for Demeanor {
    type Error = DemeanorError;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'f'|'F' => Ok(Self::Friendly),
            'n'|'N' => Ok(Self::Neutral),
            'h'|'H'|
            'a'|'A' => Ok(Self::Hostile),
            _ => Err(DemeanorError::NotDemeanor(value))
        }
    }
}

impl Default for Demeanor {
    fn default() -> Self {
        Self::Neutral
    }
}

/// Some basic/preliminary factions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum EntityFaction {
    /// Reserved for actual [Player].
    Player { pvp: bool },
    /// You don't want to piss these guys off…
    Guard { demeanor: Demeanor, general_stance: Demeanor },
    /// A non-specialist catch-all variant.
    NPC { demeanor: Demeanor },
    /// Vendors are by default more or less "neutral".
    Vendor { demeanor: Demeanor },
}

pub enum EntityFactionError {
    VoidFaction,
    NotFaction(String),
    CannotSetPlayerPVP,
    Demeanor(DemeanorError),
}

impl From<DemeanorError> for EntityFactionError { fn from(value: DemeanorError) -> Self { Self::Demeanor(value) }}

impl Display for EntityFactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CannotSetPlayerPVP => write!(f, "Only player can set themselves into Hardcore/PvP mode."),
            Self::NotFaction(x) => write!(f, "'{x}' isn't a recognized \"faction\"."),
            Self::VoidFaction => write!(f, "An empty string doesn't represent any faction whatsoever…"),
            Self::Demeanor(d) => write!(f, "Demeanor: {d:?}")
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
            Self::Guard { demeanor,..} => write!(f, "Guard{}", demeanor),
            Self::NPC { demeanor } => write!(f, "NPC{}", demeanor),
            Self::Vendor { demeanor } => write!(f, "Vendor{}", demeanor),
            Self::Player { pvp } => write!(f, "Player{}", if *pvp {" <c yellow>(PvP)</c>"} else {""}),
        }
    }
}

impl TryFrom<&str> for EntityFaction {
    type Error = EntityFactionError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() { return Err(EntityFactionError::VoidFaction) }
        let (t,d) = value.split_once(' ').unwrap_or((value, ""));
        let d = match d.chars().nth(0) {
            None => Ok(Demeanor::Neutral),
            Some(d) => Demeanor::try_from(d),
        };
        if let Err(e) = d { return Err(EntityFactionError::from(e)); }
        
        Ok(match t.chars().nth(0) {
            None => return Err(EntityFactionError::VoidFaction),
            Some(c) => match c {
                'p'|'P' => return Err(EntityFactionError::CannotSetPlayerPVP),
                'g'|'G' => Self::Guard { demeanor: d.unwrap_or_default(), general_stance: Demeanor::Neutral },
                'v'|'V' => Self::Vendor { demeanor: d.unwrap_or_else(|_| Demeanor::Friendly) },
                // `t`=[fnah] ignores separate demeanor arg `d`.
                'f'|'F' => Self::NPC { demeanor: Demeanor::Friendly },
                'n'|'N' => Self::NPC { demeanor: Demeanor::Neutral },
                'a'|'A'|
                'h'|'H' => Self::NPC { demeanor: Demeanor::Hostile },
                _ => return Err(EntityFactionError::NotFaction(t.into())),
            }
        })
    }
}
