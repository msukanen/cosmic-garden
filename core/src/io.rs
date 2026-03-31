//! I/O related stuff lives here…

use std::ops::Deref;

use once_cell::sync::OnceCell;

/// ImmutablePath to appease lazy-init file system access…
pub(crate) struct ImmutablePath; impl ImmutablePath {
    pub fn set(path: impl Into<String>) {
        let path: String = path.into();
        DATA.set(path.clone()).expect(&format!("Cannot set DATA to '{path}'!"));
    }
}

/// Deref to appease lazy-init file system access…
impl Deref for ImmutablePath {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        DATA.get().unwrap_or_else(|| {
            panic!("DATA.get() fail. DATA_PATH var not set yet? Dev, go find out why not…");
        })
    }
}

pub(super) static DATA: OnceCell<String> = OnceCell::new();
pub(crate) static DATA_PATH: ImmutablePath = ImmutablePath;
