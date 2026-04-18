//! Set a field in HelpPage.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, show_help_if_needed, validate_access};

pub struct SetCommand;

#[async_trait]
impl Command for SetCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        show_help_if_needed!(ctx, "set");

    }
}