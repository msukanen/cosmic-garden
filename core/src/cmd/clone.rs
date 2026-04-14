//! Clone things (not clowns though)!

use async_trait::async_trait;

use crate::cmd::{Command, CommandCtx};

pub struct CloneCommand;

#[async_trait]
impl Command for CloneCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {

    }
}
