//! Return to Mordor?

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, player_or_bust, tell_user};

pub struct ReturnCommand;

#[async_trait]
impl Command for ReturnCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        tell_user!(ctx.writer, "No one simply returns - or walks - or even go to Mordor, nor let hobbits run amok…\n");
    }
}
