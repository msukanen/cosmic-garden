//! IEdit tit-ler.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityMut};

pub struct TitleCommand;

#[async_trait]
impl Command for TitleCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        cmd_xedit_title!(ctx, iedit);
    }
}
