//! Abort MEdit.

use async_trait::async_trait;

use crate::cmd::{Command, CommandCtx, cmd_alias::BufferNuke};

pub struct AbortCommand;

#[async_trait]
impl Command for AbortCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        BufferNuke.exec(ctx).await;
    }
}
