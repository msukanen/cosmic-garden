//! Room environment.

use serde::{Deserialize, Serialize};

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

/// Unusual/mentionable terrain types.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Terrain {
    Slippery, // Ice, oil, etc.
    Sharp, // obsidian shards, etc.
    Underwater,
    PartialSubmerge, // not entirely underwater, but difficult to move
    Sand, // deep or otherwise
}
