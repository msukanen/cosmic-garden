//! Room editor - abort!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, io::ClientState, player_or_bust, tell_user};

pub struct AbortCommand;

/// Abort currently ongoing editing. Modifications done will *not* carry over.
#[async_trait]
impl Command for AbortCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        ctx.state = ClientState::Playing { player: plr.clone() };
        plr.write().await.redit_buffer = None;
        if !ctx.args.starts_with('q') {
            tell_user!(ctx.writer, "Edits erased. Resuming normal life…\n");
        }
    }
}
