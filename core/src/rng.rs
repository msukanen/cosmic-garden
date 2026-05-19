//! Pseudo-randoming around.

/// `seed` the "RNG"; return the new value.
#[inline] pub(crate) fn cg_rng(seed: u64) -> u64 { seed.wrapping_mul(6364136223846793005).wrapping_add(1) }
/// Generate a [0.0, 1.0] range value from give `base`.
// Perfect 1.0 is rare, but… can happen, thus the range isn't [0.0, 1.0)
#[inline] pub(crate) const fn ai_probability(base: u64) -> f32 { (base >> 32) as f32 / 4294967296.0 }
/// Check vs [0.0, 1.0] range whether to do something…
#[inline] pub(crate) const fn ai_do(base: u64, chance: f32) -> bool { ai_probability(base) <= chance }
/// Generate some random seed for the "RNG".
#[inline] pub(crate) fn cg_rng_default() -> u64 { rand::random::<u64>() }
