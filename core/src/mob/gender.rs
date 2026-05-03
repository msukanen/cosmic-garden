//! Gender of things.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum GenderType {
    Unset,
    Female,
    Male,
}

impl Display for GenderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Unset => "<c gray>(gender not set, yet)</c>",
            Self::Female => "female",
            Self::Male => "male",
        })
    }
}

#[derive(Debug)]
pub enum GenderError {
    Immutable,
    NotRecognized(String),
    VoidGender,
}

impl Display for GenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immutable => write!(f, "Gender is immutable…"),
            Self::NotRecognized(v) => write!(f, "'{v}' is not recognized as any sort of a gender."),
            Self::VoidGender => write!(f, "'nothing' isn't applicable.")
        }
    }
}

impl TryFrom<&str> for GenderType {
    type Error = GenderError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value.chars().nth(0) {
            None => return Err(GenderError::VoidGender),
            Some(c) => match c {
                'b'|'B'|// boy, etc.
                'm'|'M' // male, man, etc.
                    => Self::Male,
                
                'w'|'W'|// woman, etc.
                'f'|'F'|// female, etc.
                'g'|'G'|// girl, etc.
                'n'|'N'|// finnish: nainen, etc.
                't'|'T' // finnish: tyttö, etc.
                    => Self::Female,
                
                _ => return Err(GenderError::NotRecognized(value.into()))
            }
        })
    }
}

impl Default for GenderType {
    /// Generate "default" gender in random.
    /// 
    /// In avg. the result is ever so slightly female biased.
    // …which should reflect reality, if I read the real world gender distribution right.
    fn default() -> Self {
        if rand::random::<f32>() >= 0.5 {
            Self::Female
        } else { Self::Male }
    }
}

pub trait Gender {
    /// Get [GenderType].
    fn gender(&self) -> GenderType;
    /// Set gender to `gender`, if/when possible/relevant.
    fn set_gender(&mut self, gender: GenderType) -> Result<(), GenderError>;
}
