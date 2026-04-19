//! Various global consts and constlikes.

use std::{collections::HashSet, fs, ops::Deref, path::PathBuf, sync::Arc};

use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;

use crate::io::reserved_names_fp;

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
        let mut s = HashSet::new();
        // some OS-related things...
        for name in &[
            "con", "prn", "aux", "nul",
            "null", "dev", "root", "home",
            "usr", "etc", "var", "tmp",
        ] { s.insert(*name);}
        for i in 1..=9 {
            s.insert(Box::leak(format!("com{i}").into_boxed_str()));
            s.insert(Box::leak(format!("lpt{i}").into_boxed_str()));
        }
        // names, etc.
        for name in &[
            "admin", "sys", "system", "root",
            "world", "self", "me", "omfg",
            "room", "here", "all", "force",
            "builder", "anybuilder", "any_builder",
            "any"
        ] { s.insert(*name);}
        s
    };

    pub static ref CONFIG_RESERVED: Arc<RwLock<HashSet<String>>> = {
        let mut s = HashSet::new();
        if let Ok(buf) = fs::read_to_string(reserved_names_fp()) {
            let words = buf.split(';').map(|w| w.trim()).collect::<Vec<&str>>();
            for w in words {
                s.insert(w.into());
            }
        } else {
            log::trace!("No 'reserved.names' to process.");
        }
        Arc::new(RwLock::new(s))
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
