//! Mobs have 'factions' (Players too…).

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
