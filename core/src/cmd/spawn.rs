//! Spawn something!

use async_trait::async_trait;

use crate::cmd::{Command, CommandCtx};

pub struct SpawnCommand;

#[async_trait]
impl Command for SpawnCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {

    }
}
