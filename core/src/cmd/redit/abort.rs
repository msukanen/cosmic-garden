//! Room editor - abort!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, tell_user};

pub struct AbortCommand;

#[async_trait]
impl Command for AbortCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        log::warn!("ReditCommand unimplemented");
        tell_user!(ctx.writer, "TODO\n")// TODO
    }
}
