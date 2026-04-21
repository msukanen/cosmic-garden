//! Wield (or try to wield) something as a weapon…

use async_trait::async_trait;

use crate::cmd::{Command, CommandCtx};

pub struct WieldCommand;

#[async_trait]
impl Command for WieldCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {

    }
}
