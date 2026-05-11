//! Room environment.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub enum SpecialEnvironmentError {
    GravityClash,
    GravityModelMissing,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum MemoryFogType {
    Jail,
    Mystic,
}

/// Special environments are a bitmap.
pub type SpecialEnvironment = u64;
pub const SPECIAL_ENVIRONMENT_DEFAULT: SpecialEnvironment = 0;
// Various special environments…
pub const SPECIAL_ENVIRONMENT_GAS_TRAP: SpecialEnvironment  = 1 << 0;// suffocating
pub const SPECIAL_ENVIRONMENT_FREEZER: SpecialEnvironment   = 1 << 1;// ThermalType(Frozen or lower)
pub const SPECIAL_ENVIRONMENT_INFERNO: SpecialEnvironment   = 1 << 2;// ThermalType(Very Hot or higher); f.ex. near volcano/vast geysirs and other such
pub const SPECIAL_ENVIRONMENT_TOXIC: SpecialEnvironment     = 1 << 3;// toxin/poison
pub const SPECIAL_ENVIRONMENT_CORROSIVE: SpecialEnvironment = 1 << 4;// corrosive vapors, etc.
pub const SPECIAL_ENVIRONMENT_LOUD: SpecialEnvironment      = 1 << 5;// loud enough to make shouting absolutely necessary (if heard even then…)
pub const SPECIAL_ENVIRONMENT_STINKY: SpecialEnvironment    = 1 << 6;// well beyond "just bad smelling"; not quite Gas Trap, but close enough…
pub const SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY: SpecialEnvironment = 1 << 7;// generally high/low gravity notably differing from ye olde 1 g.
    pub const GRAVITY_ANOMALY_HIGH_H: SpecialEnvironment = SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY + 1 << 8;
    pub const GRAVITY_ANOMALY_LOW_H: SpecialEnvironment  = SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY + 1 << 9;
pub const SPECIAL_ENVIRONMENT_OBSTRUCTED_VISIBILITY: SpecialEnvironment = 1 << 10;
pub const SPECIAL_ENVIRONMENT_FOGGED_VISIBILITY: SpecialEnvironment     = 1 << 11;
// Weather stuff…
pub const WEATHER_RAIN: SpecialEnvironment = 1 << 32;
pub const WEATHER_CLEAR: SpecialEnvironment = 1 << 33;

/// Unusual/mentionable terrain types.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Terrain {
    Slippery, // Ice, oil, etc.
    Sharp, // obsidian shards, etc.
    Underwater,
    PartialSubmerge, // not entirely underwater, but difficult to move
    DeepMud, // ankle depth or (much) worse
    Sand, // deep or otherwise
}
