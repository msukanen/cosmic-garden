//! Directions, directions!

use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// Directions
#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    Up, Down,
    North, South, West, East,
    NW, NE, SW, SE,
    /// Custom directions, e.g. portals, trapdoors, etc.
    Custom(String)
}

/// Direction related error(s).
pub enum DirectionError {
    NotCardinal
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Custom(v) => v.as_str(),
            _ => self.as_str()
        })
    }
}

impl From<&str> for Direction {
    fn from(value: &str) -> Self {
        let lc = value.to_lowercase();
        match value.to_lowercase().as_str() {
            "d"|"down" => Self::Down,
            "e"|"east" => Self::East,
            "ne"|"northeast"|"north-east" => Self::NE,
            "nw"|"northwest"|"north-west" => Self::NW,
            "n"|"north" => Self::North,
            "se"|"southeast"|"south-east" => Self::SE,
            "sw"|"southwest"|"south-west" => Self::SW,
            "s"|"south" => Self::South,
            "u"|"up" => Self::Up,
            "w"|"west" => Self::West,
            _ => Self::Custom(lc)
        }
    }
}

impl From<&&str> for Direction {
    fn from(value: &&str) -> Self {
        Self::from(*value)
    }
}

impl From<&String> for Direction {
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<String> for Direction {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl Direction {
    /// Get us as cardinal clone, if possible.
    pub fn as_cardinal<'a>(&'a self) -> Result<&'a Direction, DirectionError> {
        match self {
            Self::Custom(_) => Err(DirectionError::NotCardinal),
            _ => Ok(self)
        }
    }

    /// Get opposite direction, if possible.
    pub fn opposite(&self) -> Result<Direction, DirectionError> {
        match self {
            Self::Down => Ok(Self::Up),
            Self::East => Ok(Self::West),
            Self::NE => Ok(Self::SW),
            Self::NW => Ok(Self::SE),
            Self::North => Ok(Self::South),
            Self::SE => Ok(Self::NW),
            Self::SW => Ok(Self::NE),
            Self::South => Ok(Self::North),
            Self::Up => Ok(Self::Down),
            Self::West => Ok(Self::East),
            Self::Custom(_) => Err(DirectionError::NotCardinal)
        }
    }

    pub fn as_str<'a>(&'a self) -> &'static str {
        match self {
            Self::Custom(_) => unimplemented!("No as_str for Custom..."),
            Self::Down => "down",
            Self::East => "east",
            Self::NE => "northeast",
            Self::NW => "northwest",
            Self::North => "north",
            Self::SE => "southeast",
            Self::SW => "southwest",
            Self::South => "south",
            Self::Up => "up",
            Self::West => "west"
        }
    }
}

pub trait Directional {
    fn as_cardinal(&self) -> Result<Direction, DirectionError>;
    fn opposite(&self) -> Result<Direction, DirectionError>;
}

impl Directional for &str {
    fn as_cardinal(&self) -> Result<Direction, DirectionError> {
        let dir = Direction::from(self);
        Ok(dir.as_cardinal()?.clone())
    }

    fn opposite(&self) -> Result<Direction, DirectionError> {
        let dir = Direction::from(self);
        Ok(dir.opposite()?)
    }
}

impl Directional for String {
    fn as_cardinal(&self) -> Result<Direction, DirectionError> {
        self.as_str().as_cardinal()
    }

    fn opposite(&self) -> Result<Direction, DirectionError> {
        self.as_str().opposite()
    }
}
