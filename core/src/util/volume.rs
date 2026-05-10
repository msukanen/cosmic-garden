//! Anything with volume (size)…

use crate::item::container::StorageSpace;

pub trait Volumed {
    /// Get this thing's [size][StorageSpace].
    fn size(&self) -> StorageSpace;
}

pub trait VolumeMut {
    /// Set this thing's [`sz`][StorageSpace] (size).
    fn set_size(&mut self, sz: StorageSpace) -> bool;
}
