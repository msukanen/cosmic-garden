//! Item editor - abort!
use async_trait::async_trait;
use crate::{cmd::{Command, CommandCtx, cmd_alias::AbortCommand as Bail}};
pub struct AbortCommand;

/// Route abort via [Bail][crate::cmd::cmd_alias::AbortCommand].
#[async_trait]
impl Command for AbortCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        Bail.exec(ctx).await;
    }
}
