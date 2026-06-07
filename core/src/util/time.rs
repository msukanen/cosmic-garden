//! Time related utility functions…

/// `hz` as millis.
#[inline(always)]
pub(crate) const fn hz2ms(hz: u8) -> u64 { 1_000/ hz as u64 }
