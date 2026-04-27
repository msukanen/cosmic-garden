//! Commanding core.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{cmd_alias::CMD_ALIASES, goto::GotoCommand}, edit::EditorMode, io::ClientState, player::Player, tell_user, tell_user_unk, thread::signal::SignalSenderChannels, util::direction::Directional, world::World};

#[macro_use]
pub mod cmd_alias;

// Get modules.
include!(concat!(env!("OUT_DIR"), "/registry.rs"));
// Get the commands hashmap.
include!(concat!(env!("OUT_DIR"), "/commands.rs"));

/// Command "context" to be handed across all the unified [Command] things.
pub struct CommandCtx<'a>
{
    pub pre_pad_n: bool,
    pub state: ClientState,
    pub world: Arc<RwLock<World>>,
    pub out: &'a SignalSenderChannels,
    pub args: &'a str,
    pub writer: &'a mut (dyn tokio::io::AsyncWrite + Unpin + Send),
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
            EditorMode::Redit { .. } => &REDIT_COMMANDS,
            EditorMode::Hedit { .. } => &HEDIT_COMMANDS,
            EditorMode::Iedit { .. } => &IEDIT_COMMANDS,
            EditorMode::Medit { .. } => &MEDIT_COMMANDS,
        },
        _ => {// should not happen, but…
            log::error!("Player state '{:?}' invalid for cmd processing?!", ctx.state);
            return ctx.state.clone();
        }
    };

    if ctx.pre_pad_n {
        tell_user!(ctx.writer, "\n");
        ctx.pre_pad_n = false;
    }
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

#[cfg(test)]
mod cmd_tests {
    use crate::{cmd::{Command, CommandCtx, look::LookCommand}, io::ClientState, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn synthetic_commandctx() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut mock_sock = std::io::Cursor::new(&mut buffer);
        let (world, sigs, plr, _) = get_operational_mock_world().await;
        let mut ctx = CommandCtx {
            writer: &mut mock_sock,
            args: "",
            pre_pad_n: false,
            out: &sigs.out,
            state: ClientState::Playing { player: plr.clone() },
            world: world.clone(),
        };
        LookCommand.exec(&mut ctx).await;
        log::debug!("{buffer:?}");
    }
}
