//! "Pop" something…

use async_trait::async_trait;

use crate::cmd::{Command, CommandCtx};

pub struct PopCommand;

#[async_trait]
impl Command for PopCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        
    }
}
