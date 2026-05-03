//! Mob stats.

use std::{fmt::Display, ops::{AddAssign, Div, Mul, SubAssign}};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use dicebag::InclusiveRandomRange;

use crate::{r#const::SIZE_BALANCE, item::{container::storage::StorageSpace, weapon::WeaponSize}, traits::Tickable, util::approx::ApproxI32};

pub const MAX_STAT_VALUE: StatValue = 1000.0;
// TODO: convert raw TICKS_BETWEEN_DRAIN into some runtime calibrateable type.
pub const TICKS_BETWEEN_DRAIN: StatValue = 10 as StatValue;
const UNC_THRESHOLD: StatValue = 1.0;//    HP 1
pub const DED_THRESHOLD: StatValue = -10.0;//  HP -10
const SMR_THRESHOLD: StatValue = -100.0;// HP -100

/// Various [Stat] related error states.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatError {
    NoDrain,
    NotApplicable,
    NotStat,
}

impl Display for StatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDrain => write!(f, "Stat does not support 'drain'."),
            Self::NotApplicable => write!(f, "Something or other that was attempted isn't compatible…"),
            Self::NotStat => write!(f, "That is not a stat type…"),
        }
    }
}

/// Stat value type.
pub type StatValue = f32;

impl ApproxI32 for StatValue {
    fn approx_i32(&self) -> i32 {
        let base = self.floor() as i32;
        let fract = self.fract();
        if rand::random::<f32>() < fract {
            base + 1
        } else {
            base
        }
    }
}

/// Stat types for [Stat::new].
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub enum StatType {
    HP,
    MP,
    SN,
    San,
    Str,
    Nim,
    Brn,
    Rep,
}

impl StatType {
    pub const fn display_list() -> &'static str {
        "BRN, HP, MP, NIM, REP, SAN, SN, STR"
    }
}

impl Display for StatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::HP => "HP",
            Self::MP => "MP",
            Self::SN => "SN",
            Self::Brn => "BRN",
            Self::Nim => "NIM",
            Self::Str => "STR",
            Self::San => "SAN",
            Self::Rep => "REP",
        })
    }
}

impl TryFrom<&str> for StatType {
    type Error = StatError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "hp"|"HP"|"Hp" => Ok(StatType::HP),
            "mp"|"MP"|"Mp" => Ok(StatType::MP),
            "sn"|"SN"|"Sn" => Ok(StatType::SN),
            "san"|"SAN"|"San" => Ok(StatType::San),
            "brn"|"BRN"|"Brn" => Ok(StatType::Brn),
            "nim"|"NIM"|"Nim" => Ok(StatType::Nim),
            "str"|"STR"|"Str" => Ok(StatType::Str),
            "rep"|"REP"|"Rep" => Ok(StatType::Rep),
            _ => Err(StatError::NotStat)
        }
    }
}

/// Stat core.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Stat {
    /// Hit points.
    HP { curr: StatValue, max: StatValue },
    /// Mental power. Also "mana" in some contexts.
    MP { curr: StatValue, max: StatValue, drain: StatValue },
    /// Sanity, or insanity…
    San { curr: StatValue, max: StatValue, drain: StatValue },
    /// Stamina.
    SN { curr: StatValue, max: StatValue, drain: StatValue },
    /// Braininess, IQ, etc.
    Brn { curr: StatValue, max: StatValue },
    /// Strength.
    Str { curr: StatValue, max: StatValue },
    /// Nimbleness, dexterity, etc.
    Nim { curr: StatValue, max: StatValue },
    /// Reputation. Rep doesn't have max value, but it does clamp to -100 at bottom range.
    Rep { curr: StatValue },
}

impl Stat {
    pub fn new(typ: StatType) -> Self {
        match typ {
            StatType::Brn => Self::Brn { curr: 100.0, max: 100.0 },
            StatType::HP  => Self::HP { curr: 100.0, max: 100.0 },
            StatType::Nim => Self::Nim { curr: 100.0, max: 100.0 },
            StatType::Str => Self::Str { curr: 100.0, max: 100.0 },
            StatType::Rep => Self::Rep { curr: 100.0 },
            StatType::MP  => Self::MP { curr: 100.0, max: 100.0, drain: 0.0 },
            StatType::San => Self::San { curr: 100.0, max: 100.0, drain: 0.0 },
            StatType::SN  => Self::SN { curr: 100.0, max: 100.0, drain: 0.0 },
        }
    }

    pub const fn display_list() -> &'static str { StatType::display_list() }
}

impl AddAssign<StatValue> for Stat {
    fn add_assign(&mut self, rhs: StatValue) {
        match self {
            // HP and REP clamp to [-100..max] range
            // * 1+ = alive and kicking
            // * -10+ = unconscious
            // * <-10 = dead, deader, a smear on the floor (at -100)…
            Self::HP { curr, max } => *curr = (*curr + rhs).clamp(SMR_THRESHOLD, *max),
            
            // Rep min: -100, no max.
            Self::Rep { curr } => *curr = (*curr + rhs).max(-100.0),
            
            // others clamp to 0..max range
            Self::Brn { curr, max, ..} |
            Self::MP { curr, max, ..}  |
            Self::Nim { curr, max, ..} |
            Self::Str { curr, max, ..} |
            Self::SN { curr, max, ..}  |
            Self::San { curr, max, ..} => *curr = (*curr + rhs).clamp(0.0, *max)
        }
    }
}

impl SubAssign<StatValue> for Stat {
    #[inline]
    fn sub_assign(&mut self, rhs: StatValue) {
        self.add_assign(-rhs);
    }
}

impl AddAssign<i32> for Stat {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        self.add_assign(rhs as StatValue);
    }
}

impl SubAssign<i32> for Stat {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        self.sub_assign(rhs as StatValue);
    }
}

impl Div<StatValue> for Stat {
    type Output = StatValue;
    fn div(self, rhs: StatValue) -> Self::Output { (&self) / rhs }
}
impl Div<StatValue> for &Stat {
    type Output = StatValue;
    fn div(self, rhs: StatValue) -> Self::Output { self.current() / rhs }
}

impl Stat {
    /// Set [Stat] max `value`.
    /// 
    /// Note that max `value` *cannot* be negative (will be forced to be 0+)
    /// and that it cannot exceed [MAX_STAT_VALUE] (will be clamped if necessary).
    /// [Stat::San] max value cannot exceed `100.0` (=absolutely clear sane).
    /// 
    /// We pump current value up by delta between new and old max if max is increased.
    pub fn set_max(&mut self, value: StatValue) -> &mut Self {
        let value = value.abs().min(MAX_STAT_VALUE);
        let delta = value - self.max();
        match self {
            Self::HP { max, ..}  |
            Self::Brn { max, ..} |
            Self::Nim { max, ..} |
            Self::Str { max, ..} |
            Self::MP { max, ..}  |
            Self::SN { max, ..}  => *max = value,
            Self::San { max, ..} => *max = value.min(100.0),

            Self::Rep { .. } => ()
        }

        if self.current() > self.max() {
            self.set_curr(self.max());
        } else if delta > 0.0 {
            self.set_curr(self.current() + delta);
        }

        self
    }

    /// Set [Stat] drain `value`.
    /// 
    /// # Notes
    /// * it isn't an error to try set drain of a [Stat] which doesn't support it,
    ///   the `value` in such cases is simply ignored.
    /// * drain *can* be positive, in which case the [Stat] *gains* `curr`.
    /// * drain *cannot* exceed 1/[TICKS_BETWEEN_DRAIN]th of [Stat]'s `max`.
    /// 
    /// # Args
    /// - `value` to be used as the new drain figure.
    /// 
    pub fn set_drain(&mut self, value: StatValue) -> &mut Self {
        match self {
            Self::MP { drain, max, ..} |
            Self::San { max, drain, ..}|
            Self::SN { max, drain, ..}
            => {
                let abs_drain = value.abs().min(*max / TICKS_BETWEEN_DRAIN);
                *drain = if value >= 0.0 { abs_drain } else { -abs_drain }
            },

            // drainless [Stat] simply fall through.
            _ => ()
        }
        self
    }

    /// Set [Stat] current `value`.
    /// 
    /// # Notes
    /// * `value` *cannot* exceed [Stat::max].
    pub fn set_curr(&mut self, value: StatValue) -> &mut Self{
        match self {
            Self::HP { curr, max }    => *curr = value.clamp(SMR_THRESHOLD, *max),
            Self::MP { curr, max, ..} |
            Self::San { curr, max, ..}|
            Self::SN { curr, max, ..} |
            Self::Str { curr, max, ..}|
            Self::Brn { curr, max, ..}|
            Self::Nim { curr, max, ..}=> *curr = value.clamp(0.0, *max),
            Self::Rep { curr } => *curr = value.max(-100.0),
        }
        self
    }

    /// Get [Stat] max value.
    pub fn max(&self) -> StatValue {
        match self {
            Self::HP { max, ..}  |
            Self::MP { max, ..}  |
            Self::Brn { max, ..} |
            Self::Nim { max, ..} |
            Self::SN { max, ..}  |
            Self::Str { max, ..} |
            Self::San { max, ..} => *max,
            Self::Rep { .. } => StatValue::MAX,
        }
    }

    /// We're already capped?
    pub fn capped(&self) -> bool {
        self.current() >= self.max()
    }

    /// Get [Stat] current value.
    pub fn current(&self) -> StatValue {
        match self {
            Self::HP { curr, ..} |
            Self::MP { curr, ..} |
            Self::Brn { curr, ..}|
            Self::Nim { curr, ..}|
            Self::SN { curr, ..} |
            Self::Str { curr, ..}|
            Self::Rep { curr}    |
            Self::San { curr, ..}=> *curr
        }
    }

    /// Get [Stat] drain value, if applicable.
    pub fn drain(&self) -> Result<StatValue, StatError> {
        match self {
            Self::MP { drain, ..}  |
            Self::SN { drain, ..}  |
            Self::San { drain, ..} => Ok(*drain),
            
            _ => Err(StatError::NoDrain)
        }
    }

    /// Get [Stat] drain value, defaulting to `0.0` for those [Stat] which do not actually support it.
    pub fn drain_or_zero(&self) -> StatValue {
        self.drain().unwrap_or(0.0)
    }

    /// [Stat] tells whether you're dead or not, if/when applicable…
    pub fn is_dead(&self) -> Result<bool, StatError> {
        match self {
            Self::HP { curr, .. } => Ok(*curr < DED_THRESHOLD),
            _ => Err(StatError::NotApplicable)
        }
    }

    /// [Stat] tells whether you're unconscious or not, if/when applicable…
    pub fn is_unconscious(&self) -> Result<bool, StatError> {
        match self {
            Self::HP { curr, ..} => Ok(*curr < UNC_THRESHOLD && !self.is_dead().unwrap()),
            Self::MP { curr, ..} => Ok(*curr <= 0.0),
            _ => Err(StatError::NotApplicable)
        }
    }

    /// Get [sanity][StatType::San] based speech coherency delta that can
    /// be used as e.g. text scrambling weight value, etc.
    /// 
    /// # Returns
    /// Some delta value within \[0..1].
    pub fn rel_speech_coherency_delta(&self, other_san: &Stat) -> f32 {
        match (self, other_san) {
            (Self::San { .. }, Self::San { .. }) => {
                // get delta as 0..1 range
                (self.current() - other_san.current()).abs() / 100.0
            },
            
            // a no-op for all but SAN.
            _ => 0.0
        }
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // HP: (12/34)
            Self::HP { curr, max } => write!(f, "HP: ({:.0}/{:.0})", curr, max),
            Self::Brn { curr, max } => write!(f, "BRN: ({:.0}/{:.0})", curr, max),
            Self::Nim { curr, max } => write!(f, "NIM: ({:.0}/{:.0})", curr, max),
            Self::Str { curr, max } => write!(f, "STR: ({:.0}/{:.0})", curr, max),

            // REP: (+100.0)
            Self::Rep { curr } => write!(f, "REP: ({:+.1})", curr),
            
            // MP: (12/34)[-1.1]
            // MP: (12/34)
            Self::MP { curr, max, drain } => if *drain != 0.0 {
                write!(f, "MP: ({:.0}/{:.0})[{:+.1}/t]", curr, max, drain)
            } else {
                write!(f, "MP: ({:.0}/{:.0})", curr, max)
            }

            // SN: (12/34)[-1.1]
            // SN: (12/34)
            Self::SN { curr, max, drain } => if *drain != 0.0 {
                write!(f, "SN: ({:.0}/{:.0})[{:+.1}/t]", curr, max, drain)
            } else {
                write!(f, "SN: ({:.0}/{:.0})", curr, max)
            }

            // San: (12/34)
            // Sun: ($@#?!)
            Self::San { curr, max, ..} => {
                if *curr < *max * 0.1 {
                    let s = 'A'..='Z';
                    let r = 'a'..='z';
                    write!(f, "{}{}{}: ($@#?!…", s.random_of(), r.random_of(), r.random_of())
                } else {
                    write!(f, "San: ({:.0}/{:.0})", curr, max)
                }
            }
        }
    }
}

#[async_trait]
impl Tickable for Stat {
    async fn tick(&mut self) -> bool {
        let Ok(drain) = self.drain() else {
            // no drain, nothing to tick
            return false;
        };
        if drain.abs() < 0.001 {
            // no meaningful drain; nothing to tick
            return false;
        }
        if self.capped() && drain > 0.0 {
            return false;
        }
        let old = self.current();
        self.add_assign(drain);
        (self.current() - old).abs() > 0.001
    }
}

impl PartialEq<StatValue> for Stat {
    fn eq(&self, other: &StatValue) -> bool {
        (self.current() - other).abs() < 0.001
    }
}

impl PartialEq<&Stat> for StatValue {
    fn eq(&self, other: &&Stat) -> bool {
        <Stat as PartialEq<StatValue>>::eq(*other, self)
    }
}

impl PartialEq<Stat> for StatValue {
    fn eq(&self, other: &Stat) -> bool {
        <Stat as PartialEq<StatValue>>::eq(other, self)
    }
}

impl PartialEq<StatValue> for &Stat {
    fn eq(&self, other: &StatValue) -> bool {
        <Stat as PartialEq<StatValue>>::eq(*self, other)
    }
}

impl PartialEq<i32> for Stat {
    fn eq(&self, other: &i32) -> bool {
        <Stat as PartialEq<StatValue>>::eq(self, &(*other as StatValue))
    }
}

impl PartialEq<i32> for &Stat {
    fn eq(&self, other: &i32) -> bool {
        <Stat as PartialEq<i32>>::eq(*self, other)
    }
}

impl Mul<&Stat> for f32 {
    type Output = Self;
    fn mul(self, rhs: &Stat) -> Self::Output {
        self * rhs.current()
    }
}

impl Mul<&WeaponSize> for StatValue {
    type Output = StatValue;
    fn mul(self, rhs: &WeaponSize) -> Self::Output {
        self / (rhs.required_space() / SIZE_BALANCE as StorageSpace) as StatValue
    }
}

#[cfg(test)]
mod stat_tests {
    use super::*;

    #[test]
    fn clamping_works() {
        let mut hp = Stat::new(StatType::HP);
        assert_eq!(100.0, hp.current());
        assert_eq!(100.0, hp.max());

        // max drops current if curr > new max
        hp.set_max(95.0);// 95
        assert_eq!(95.0, hp.current());

        // see that max holds
        hp += 5; // curr 95
        assert_eq!(95.0, hp.current());

        // negative works
        hp -= 100; // curr -5
        assert_eq!(-5.0, hp.current());
        let Ok(true) = hp.is_unconscious() else {
            panic!("Not unconscious? Math fail!")
        };
        hp -= 5; // curr -10
        // -10 should not be dead yet
        let Ok(false) = hp.is_dead() else {
            panic!("Uh oh. Dead too soon!");
        };
        // -11 oughta kill
        hp -= 1; // curr -11
        assert_eq!(DED_THRESHOLD - 1.0, hp.current());
        let Ok(true) = hp.is_dead() else {
            panic!("What gives? They're still kicking?!");
        };

        // see that we can't reduce them into negative singularity…
        hp -= 100; // curr -100
        assert_eq!(SMR_THRESHOLD, hp.current());
    }

    /// Enforce that StatType enum count and its display_list() are kept in strict sync.
    #[test]
    fn stat_display_list_is_in_sync() {
        assert_eq!("BRN, HP, MP, NIM, REP, SAN, SN, STR",
            StatType::display_list(),
            "Update StatType::display_list()! Out of sync.");
        trait StatKill {
            fn check_it(&self) -> bool;
        }
        impl StatKill for StatType {
            fn check_it(&self) -> bool {
                match self {
                Self::HP |
                Self::MP |
                Self::SN |
                Self::Brn |
                Self::Nim |
                Self::Str |
                Self::Rep |
                Self::San => true,
                }
            }
        }
        let x = StatType::HP;
        assert!(true == x.check_it());
    }
}
