//! Room tit-ler.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityMut, player_or_bust, show_help_if_needed, tell_user};

pub struct TitleCommand;

#[async_trait]
impl Command for TitleCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        show_help_if_needed!(ctx, "redit-title");

        let mut lock = plr.write().await;
        let Some(ref mut room) = lock.redit_buffer else {
            tell_user!(ctx.writer, "Something weird in the neighborhood…\n");
            return;
        };
        room.set_title(ctx.args);
        tell_user!(ctx.writer, "Shadow buffer title set to: {}\n", ctx.args);
        ctx.state.set_dirty(true);
    }
}