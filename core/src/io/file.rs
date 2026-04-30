//! File access related stuff.

use std::path::PathBuf;

use crate::{r#const::{BP_PATH, ENTITY_BP_PATH, HELP_PATH, ROOM_PATH, SAVE_PATH, WORLD_ID, WORLD_PATH}, identity::uniq::StrUuid};
const EXT_BLUEPRINT: &'static str = ".blueprint";
const EXT_SAVE: &'static str = ".player";
const EXT_USER: &'static str = ".user";
const EXT_HELP: &'static str = ".help";
const EXT_LIB:  &'static str = ".library";
const EXT_WORLD: &'static str = ".world";
const EXT_ROOM:  &'static str = ".room";
const EXT_ENT_BP: &'static str = ".entity";

#[inline] pub fn user_save_fp(owner: &str) -> PathBuf {
    SAVE_PATH.join(format!("{owner}{EXT_USER}"))
}

#[inline] pub fn player_save_fp(owner: &str, id: &str) -> PathBuf {
    SAVE_PATH.join(format!("{owner}-{id}{EXT_SAVE}"))
}

#[inline] pub fn help_entry_fp(id: &str) -> PathBuf {
    HELP_PATH.join(format!("{}{EXT_HELP}", id.show_uuid(false)))
}

#[inline] pub fn help_lib_fp() -> PathBuf {
    HELP_PATH.join(format!("{}{EXT_LIB}", WORLD_ID.as_str()))
}

#[inline] pub fn world_fp() -> PathBuf {
    WORLD_PATH.join(format!{"{}{EXT_WORLD}", WORLD_ID.as_str()})
}

#[inline] pub fn cmd_alias_fp() -> PathBuf {
    WORLD_PATH.join("cmd_alias.json")
}

#[inline] pub fn blueprint_lib_fp() -> PathBuf {
    BP_PATH.join(format!("{}{EXT_LIB}", WORLD_ID.as_str()))
}

#[inline] pub fn blueprint_entry_fp(id: &str) -> PathBuf {
    BP_PATH.join(format!("{}{EXT_BLUEPRINT}", id.show_uuid(false)))
}

#[inline ] pub fn room_fp(id: &str) -> PathBuf {
    ROOM_PATH.join(format!("{}{EXT_ROOM}", id.show_uuid(false)))
}

#[inline] pub fn reserved_names_fp() -> PathBuf {
    WORLD_PATH.join("reserved.names")
}

#[inline] pub fn entity_lib_fp() -> PathBuf {
    ENTITY_BP_PATH.join(format!("{}{EXT_LIB}", WORLD_ID.as_str()))
}

#[inline] pub fn entity_entry_fp(id: &str) -> PathBuf {
    ENTITY_BP_PATH.join(format!("{}{EXT_ENT_BP}", id.show_uuid(false)))
}
