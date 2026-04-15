//! Various global consts and constlikes.

use std::{collections::HashSet, fs, ops::Deref, path::PathBuf, sync::Arc};

use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;

// some const to deal with [World]-specific choices that aren't present for a reason or other…
pub const GREETING: &'static str = "Welcome to Cosmic Garden!";
pub const PROMPT_LOGIN: &'static str = "Login: ";
pub(super) static DATA: OnceCell<String> = OnceCell::new();
static DATA_PATH: ImmutablePath = ImmutablePath;
pub(super) static WORLD: OnceCell<String> = OnceCell::new();
pub static WORLD_ID: WorldId = WorldId;

lazy_static! {
    pub(crate) static ref WORLD_PATH: PathBuf = {
        let w_path = PathBuf::from(format!("{}/{}", *DATA_PATH, *WORLD_ID));
        fs::create_dir_all(&w_path);
        w_path
    };
    pub(crate) static ref SAVE_PATH: PathBuf = {
        let s_path = WORLD_PATH.join("save");
        fs::create_dir(&s_path);
        s_path
    };
    pub(crate) static ref BP_PATH: PathBuf = {
        let bp_path = WORLD_PATH.join("blueprint");
        fs::create_dir(&bp_path);
        bp_path
    };
    pub(crate) static ref HELP_PATH: PathBuf = {
        let help_path = WORLD_PATH.join("help");
        fs::create_dir(&help_path);
        help_path
    };
    pub(crate) static ref ROOM_PATH: PathBuf = {
        let r_path = WORLD_PATH.join("room");
        fs::create_dir(&r_path);
        r_path
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
        ] { s.insert(*name);}
        s
    };

    pub static ref CONFIG_RESERVED: Arc<RwLock<HashSet<String>>> = {
        let mut s = HashSet::new();
        if let Ok(buf) = fs::read_to_string(format!("{}/reserved.names", *DATA_PATH)) {
            let words = buf.split(';').map(|w| w.trim()).collect::<Vec<&str>>();
            for w in words {
                s.insert(w.into());
            }
        } else {
            log::trace!("No {}/reserved.names to process.", *DATA_PATH);
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
