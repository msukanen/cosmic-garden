//! Various global consts and constlikes.

use std::{collections::HashSet, fs, ops::Deref, path::PathBuf};

use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use unicode_normalization::UnicodeNormalization;

use crate::{identity::MAX_ID_LEN, io::reserved_names_fp, item::container::storage::StorageSpace, util::escape_hatch::VILLAIN_ID};

/// CPU cores in the server (at least if user so says)…
pub(crate) const CPU_CORES: usize = match option_env!("GARDEN_CORES") {
    Some(s) => usize_from_str(s),
    None => 16
};
/// Very basic `usize` "parse" specifically for `CPU_CORES`…
const fn usize_from_str(s:&str) -> usize {
    let bytes = s.as_bytes();
    let mut result = 0;
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte >= b'0' && byte <= b'9' {
            result = result * 10 + (byte - b'0') as usize;
        } else {
            panic!("Non-digit character in GARDEN_CORES!");
        }
        i += 1;
    }
    result
}

// some const to deal with [World]-specific choices that aren't present for a reason or other…
pub const GREETING: &'static str = "Welcome to Cosmic Garden!";
pub const PROMPT_LOGIN: &'static str = "Login: ";

// Data/World paths … upon which everything else hinges.
pub(super) static DATA: OnceCell<String> = OnceCell::new();
static DATA_PATH: ImmutablePath = ImmutablePath;
pub(super) static WORLD: OnceCell<String> = OnceCell::new();
pub static WORLD_ID: WorldId = WorldId;

macro_rules! maybe_fs_issue {
    ($what:ident, $fn:expr) => {
        if let Err(e) = $fn {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                log::error!("FATAL! {e:?} with {}", $what.display());
                panic!("FATAL! {e:?} with {}", $what.display());
            }
        }
    };
}

lazy_static! {
    pub(crate) static ref WORLD_PATH: PathBuf = {
        let path = std::path::Path::new(DATA_PATH.as_str()).join(WORLD_ID.as_str());
        maybe_fs_issue!(path, fs::create_dir_all(&path));
        path
    };
    pub(crate) static ref SAVE_PATH: PathBuf = {
        let path = WORLD_PATH.join("save");
        maybe_fs_issue!(path, fs::create_dir(&path));
        path
    };
    pub(crate) static ref BP_PATH: PathBuf = {
        let path = WORLD_PATH.join("blueprint");
        maybe_fs_issue!(path, fs::create_dir(&path));
        path
    };
    pub(crate) static ref HELP_PATH: PathBuf = {
        let path = WORLD_PATH.join("help");
        maybe_fs_issue!(path, fs::create_dir(&path));
        path
    };
    pub(crate) static ref ROOM_PATH: PathBuf = {
        let path = WORLD_PATH.join("room");
        maybe_fs_issue!(path, fs::create_dir(&path));
        path
    };
    pub(crate) static ref ENTITY_BP_PATH: PathBuf = {
        let path = WORLD_PATH.join("entities");
        maybe_fs_issue!(path, fs::create_dir(&path));
        path
    };

    /// Immutable [IdError::ReservedName] sources.
    pub static ref HARDCODED_RESERVED: HashSet<&'static str> = {
        let mut reserved = HashSet::new();
        
        for name in &[
            "con", "prn", "aux", "nul",
            "null", "dev", "root", "home",
            "usr", "etc", "var", "tmp",
            "bin", "www", "windows", "win32",
            "win64", "dos", "user", "ftp",
            "tcp", "http", "https", "udp",
            "admin", "sys", "system",
            "world", "self", "me", "omfg",
            "room", "here", "all", "force",
            "builder", "anybuilder", "any_builder",
            "any", "new", "actual",
        ] { reserved.insert(*name);}
        for i in 1..=9 {
            reserved.insert(Box::leak(format!("com{i}").into_boxed_str()));
            reserved.insert(Box::leak(format!("lpt{i}").into_boxed_str()));
        }
        
        // reserved "villain" words, e.g. Rust keywords et al.
        for name in VILLAIN_ID { reserved.insert(name); }

        // let's treat reserved names as a plaintext file with any non-alphanum as separator
        match fs::read_to_string(reserved_names_fp()) {
            Ok(plaintext) => {
                for raw in plaintext.split_whitespace() {
                    let cleaned: String = raw
                        .to_lowercase()
                        .nfd()
                        .filter(|c| c.is_ascii_alphanumeric())
                        .collect();
                    if !cleaned.is_empty() && cleaned.len() < MAX_ID_LEN {
                        let static_str: &'static str = Box::leak(cleaned.into_boxed_str());
                        reserved.insert(static_str);
                    }
                }
            }
            Err(e) => log::warn!("Error reading '{}': {e:?}", reserved_names_fp().display())
        }
        
        reserved
    };
}

/// ImmutablePath to appease lazy-init file system access…
pub(crate) struct ImmutablePath;// impl ImmutablePath {
//     // TODO not sure if this set() is of any use ...
//     #[allow(dead_code)]
//     pub fn set(path: impl Into<String>) {
//         let path: String = path.into();
//         DATA.set(path.clone()).expect(&format!("Cannot set DATA to '{path}'!"));
//     }
// }
/// Deref to appease lazy-init file system access…
impl Deref for ImmutablePath {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        DATA.get().unwrap_or_else(|| {
            panic!("DATA.get() fail. DATA_PATH var not set yet? Dev, go find out why not…");
        })
    }
}

pub(crate) struct WorldId;// impl WorldPath {
//     #[allow(dead_code)]
//     pub fn set(path: impl Into<String>) {
//         let path: String = path.into();
//         WORLD.set(path.clone()).expect(&format!("Cannot set WORLD to '{path}'!"));
//     }
// }
impl Deref for WorldId {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        WORLD.get().unwrap_or_else(|| {
            panic!("WORLD.get() fail! Dev, go find out why not…");
        })
    }
}

//
// Item related constants.
//
pub const SIZE_BALANCE: StorageSpace = 10;
pub const TINY_ITEM: StorageSpace = 1 * SIZE_BALANCE;
pub const SMALL_ITEM: StorageSpace = 2 * SIZE_BALANCE;
pub const MEDIUM_ITEM: StorageSpace = 4 * SIZE_BALANCE;
pub const LARGE_ITEM: StorageSpace = 7 * SIZE_BALANCE;
pub const HUGE_ITEM: StorageSpace = 12 * SIZE_BALANCE;

//
// Broadcast::FooBarBaz `~foo~` replacers.
//
pub const BCAST_FMT_ENTITY_TITLE: &'static str = "~e~";
const BCAST_REQ_BITS: usize = 12;
#[cfg(not(feature = "stresstest"))] pub const ESTIMATED_BCAST_REQ_COUNT: usize = 1 << BCAST_REQ_BITS;
#[cfg(feature = "stresstest")] pub const ESTIMATED_BCAST_REQ_COUNT: usize = 1 << (BCAST_REQ_BITS + 4);
