//! Room editor!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, tell_user};

pub mod abort;

pub struct ReditCommand;

#[async_trait]
impl Command for ReditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        log::warn!("ReditCommand unimplemented");
        tell_user!(ctx.writer, "TODO\n")// TODO
    }
}
