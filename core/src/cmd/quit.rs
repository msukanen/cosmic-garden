//! Quitter!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, io::ClientState};

pub struct QuitCommand;

#[async_trait]
impl Command for QuitCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        // nothing terribly complicated…
        ctx.state = ClientState::Logout
    }
}
