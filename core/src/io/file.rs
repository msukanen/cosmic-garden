//! File access related stuff.

use std::path::PathBuf;

use crate::{r#const::{BP_PATH, HELP_PATH, SAVE_PATH, WORLD_ID, WORLD_PATH}, string::StrUuid};
const EXT_BLUEPRINT: &'static str = ".blueprint";
const EXT_SAVE: &'static str = ".player";
const EXT_USER: &'static str = ".user";
const EXT_HELP: &'static str = ".help";
const EXT_LIB:  &'static str = ".library";
const EXT_WORLD: &'static str = ".world";
const EXT_ROOM:  &'static str = ".room";

#[inline]
pub fn user_save_fp(owner: &str) -> PathBuf {
    SAVE_PATH.join(format!("{owner}{EXT_USER}"))
}

#[inline]
pub fn player_save_fp(owner: &str, id: &str) -> PathBuf {
    SAVE_PATH.join(format!("{owner}-{id}{EXT_SAVE}"))
}

#[inline]
pub fn help_entry_fp(id: &str) -> PathBuf {
    HELP_PATH.join(format!("{}{EXT_HELP}", id.show_uuid(false)))
}

#[inline]
pub fn help_lib_fp() -> PathBuf {
    HELP_PATH.join(format!("{}{EXT_LIB}", WORLD_ID.as_str()))
}

#[inline]
pub fn world_fp() -> PathBuf {
    WORLD_PATH.join(format!{"{}{EXT_WORLD}", WORLD_ID.as_str()})
}

#[inline]
pub fn cmd_alias_fp() -> PathBuf {
    WORLD_PATH.join("cmd_alias.json")
}

#[inline]
pub fn blueprint_lib_fp() -> PathBuf {
    BP_PATH.join(format!("{}{EXT_LIB}", WORLD_ID.as_str()))
}

#[inline]
pub fn blueprint_entry_fp(id: &str) -> PathBuf {
    BP_PATH.join(format!("{}{EXT_BLUEPRINT}", id.show_uuid(false)))
}
