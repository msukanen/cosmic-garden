//! Rename/Re-ID a mob.

use async_trait::async_trait;

use crate::cmd::{Command, CommandCtx};

pub struct RenameCommand;

#[async_trait]
impl Command for RenameCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {

    }
}
