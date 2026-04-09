//! Help editor!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, tell_user};

pub mod abort;

pub struct HeditCommand;

#[async_trait]
impl Command for HeditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        log::warn!("HeditCommand unimplemented");
        tell_user!(ctx.writer, "TODO\n")// TODO
    }
}
