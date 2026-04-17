use async_trait::async_trait;
use crate::{cmd::{Command, CommandCtx}};

pub struct DescCommand;

#[async_trait]
impl Command for DescCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        cmd_xedit_desc!(self, ctx, iedit, "IEdit");
    }
}
