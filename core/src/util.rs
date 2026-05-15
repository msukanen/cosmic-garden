//! Utils for utilizing.
pub mod access;
pub mod activity;
pub mod approx;
pub mod config;
pub mod direction;
pub(crate) mod escape_hatch;
//pub mod intentqueue; pub use intentqueue::*;
pub mod translocate;
pub mod volume; pub use volume::{Volumed, VolumeMut};

#[inline]
pub const fn mem_as_gmk(bytes: f64) -> (f64, f64, f64) {
    let kib = bytes as f64 / 1024.0;
    let mib = kib / 1024.0;
    let gib = mib / 1024.0;
    (kib, mib, gib)
}
