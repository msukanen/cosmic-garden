//! Commanding core.

use std::{borrow::Cow, sync::Arc};

use async_trait::async_trait;
use tokio::{net::tcp::OwnedWriteHalf, sync::{RwLock, broadcast}};

use crate::{cmd::{cmd_alias::CMD_ALIASES, goto::GotoCommand}, edit::EditorMode, io::{Broadcast, ClientState}, player::Player, tell_user_unk, util::direction::Directional, world::World};

pub mod cmd_alias;
mod dummy;

mod dig;
mod goto;
pub mod help;
mod invis;
mod look;
mod hedit;
mod quit;
mod redit;
mod say;

pub struct CommandCtx<'a> {
    pub state: ClientState,
    pub world: Arc<RwLock<World>>,
    pub tx: &'a broadcast::Sender<Broadcast>,
    pub args: &'a str,
    pub writer: &'a mut OwnedWriteHalf,
}

impl CommandCtx<'_> {
    pub fn get_player_arc(&self) -> Option<Arc<RwLock<Player>>> {
        match &self.state {
            ClientState::Editing { player, .. } |
            ClientState::Playing { player } => Some(player.clone()),
            _ => None
        }
    }
}

/// An async trait for all commands to obey.
#[async_trait]
pub trait Command: Send + Sync {
    /// Do something…
    /// 
    /// # Args
    /// - `ctx`— CommandCtx
    async fn exec(&self, ctx: &mut CommandCtx<'_>);
}

// Get the commands hashmap.
include!(concat!(env!("OUT_DIR"), "/commands.rs"));

/// Parse part of player input and exec the corresponding command.
/// 
/// # Args
/// - `ctx`— [CommandCtx] synthetic or otherwise.
pub async fn parse_and_exec<'a>(mut ctx: CommandCtx<'_>) -> ClientState {
    if ctx.args.is_empty() {
        return ctx.state.clone();
    }

    // Spec handling:
    // - emotes between '*'s.
    // - '?' in front of a command routes via "help".
    let (cmd, args) =
        if ctx.args.starts_with('*') && ctx.args.ends_with('*') {
            ("emote", ctx.args[1..ctx.args.len()-1].trim())
        } else if ctx.args.starts_with('?') {
            ("help", ctx.args[1..].trim())
        } else {
            ctx.args.split_once(' ').unwrap_or((ctx.args.trim(), ""))
        };
    ctx.args = args;
    let table = match ctx.state {
        ClientState::Playing { .. } => &COMMANDS,
        ClientState::Editing { ref mode, .. } => match mode {
            EditorMode::Room{..} => &REDIT_COMMANDS,
            EditorMode::Help{..} => &HEDIT_COMMANDS,
        },
        _ => {// should not happen, but…
            log::error!("Player state '{:?}' invalid for cmd processing?!", ctx.state);
            return ctx.state.clone();
        }
    };

    let cmd = cmd.to_lowercase();
    if let Some(cmd) = table.get(&cmd) {
        cmd.exec(&mut ctx).await
    } else if let Some(cmd) = COMMANDS.get(&cmd) {
        cmd.exec(&mut ctx).await
    } else if let Some(cmd_alias) = CMD_ALIASES.get(&cmd) {
        // command alias was found, now to get the actual command…
        if let Some(cmd) = table.get(cmd_alias) {
            cmd.exec(&mut ctx).await
        } else if let Some(cmd) = COMMANDS.get(cmd_alias) {
            cmd.exec(&mut ctx).await
        } else {
            log::error!("Command alias '{cmd}' was mapped for '{cmd_alias}', but '{cmd_alias}' was NOT found?!");
            tell_user_unk!(ctx.writer);
        }
    } else if let Ok(dir) = cmd.as_cardinal() {
        GotoCommand.exec({ctx.args = dir.as_str(); &mut ctx}).await
    } else {
        tell_user_unk!(ctx.writer);
    }

    ctx.state.clone()
}
