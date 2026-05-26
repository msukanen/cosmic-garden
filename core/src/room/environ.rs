//! Room environment.

use serde::{Deserialize, Serialize};

use crate::mob::StatValue;

#[derive(Debug, Clone, Copy)]
pub enum SpecialEnvironmentError {
    GravityClash,
    GravityModelMissing,
    WeatherClash,
    SoundClash,
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
pub const SPECIAL_ENVIRONMENT_SOUNDLESS: SpecialEnvironment = 1 << 6;// utterly soundless - can't hear even own breath…
    pub const MASK_SP_SOUND: SpecialEnvironment
        =   SPECIAL_ENVIRONMENT_LOUD|
            SPECIAL_ENVIRONMENT_SOUNDLESS;
pub const SPECIAL_ENVIRONMENT_STINKY: SpecialEnvironment    = 1 << 8;// well beyond "just bad smelling"; not quite Gas Trap, but close enough…
pub const SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY: SpecialEnvironment = 1 << 9;// generally high/low gravity notably differing from ye olde 1 g.
    pub const GRAVITY_ANOMALY_HIGH_G: SpecialEnvironment    = 1 << 10;
    pub const GRAVITY_ANOMALY_LOW_G: SpecialEnvironment     = 1 << 11;
    pub const GRAVITY_ANOMALY_ZERO_G: SpecialEnvironment    = 1 << 12;
    pub const MASK_SP_ENV_GRAVITY: SpecialEnvironment
        =   SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY|
            GRAVITY_ANOMALY_HIGH_G|
            GRAVITY_ANOMALY_LOW_G|
            GRAVITY_ANOMALY_ZERO_G;
pub const SPECIAL_ENVIRONMENT_OBSTRUCTED_VISIBILITY: SpecialEnvironment = 1 << 13;
pub const SPECIAL_ENVIRONMENT_FOGGED_VISIBILITY: SpecialEnvironment     = 1 << 14;
// Weather stuff…
pub const WEATHER_RAIN: SpecialEnvironment  = 1 << 32;
pub const WEATHER_CLEAR: SpecialEnvironment = 1 << 33;
pub const WEATHER_STORM: SpecialEnvironment = 1 << 34;
    pub const MASK_SP_WEATHER: SpecialEnvironment
        =   WEATHER_CLEAR|
            WEATHER_RAIN;

/// Set special env bit`mask`.
/// 
/// # Args
/// - `special_environment` to manipulate in place.
/// - `mask` to set.
#[must_use = "Clashing bitmask may result in `Err`."]
pub fn set_special_env_bitmask(special_environment: &mut SpecialEnvironment, mask: SpecialEnvironment) -> Result<(), SpecialEnvironmentError> {
    // see about gravity issues…
    let mut working_env = *special_environment;

    if mask & MASK_SP_ENV_GRAVITY != 0 {
        let isolated_g = mask & MASK_SP_ENV_GRAVITY;
        working_env = (working_env & !MASK_SP_ENV_GRAVITY)
            |
            match isolated_g.count_ones() {
                0 => 0, // normal g
                1 => return Err(SpecialEnvironmentError::GravityModelMissing),
                2 => {
                    if isolated_g & SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY == 0 { return Err(SpecialEnvironmentError::GravityClash) }
                    isolated_g
                },
                _ => return Err(SpecialEnvironmentError::GravityClash)
            };
    }

    if mask & MASK_SP_WEATHER != 0 {
        let isolated_w = mask & MASK_SP_WEATHER;
        working_env = (working_env & !MASK_SP_WEATHER)
            |
            match isolated_w.count_ones() {
                0 => 0,
                1 => isolated_w,
                _ => return Err(SpecialEnvironmentError::WeatherClash)
            };
    }

    if mask & MASK_SP_SOUND != 0 {
        let isolated_s = mask & MASK_SP_SOUND;
        working_env = (working_env & !MASK_SP_SOUND)
            |
            match isolated_s.count_ones() {
                0 => 0,
                1 => isolated_s,
                _ => return Err(SpecialEnvironmentError::SoundClash)
            };
    }

    const BLEND_MASK: SpecialEnvironment =
        MASK_SP_ENV_GRAVITY|
        MASK_SP_SOUND|
        MASK_SP_WEATHER;
    *special_environment = working_env | (mask & !BLEND_MASK);

    Ok(())
}

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

#[derive(Debug, Clone, Copy)]
pub enum EnvironmentEvent {
    InFlames,
    Explosion { magnitude: StatValue },
    GasLeak { toxicity: f32 },
    StructuralFailure,
    MagicalSurge { intensity: f32 },
}
