//! Weave buffered item into existence!

use async_trait::async_trait;

use crate::cmd::{Command, CommandCtx};

pub struct WeaveCommand;

#[async_trait]
impl Command for WeaveCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {

    }
}
