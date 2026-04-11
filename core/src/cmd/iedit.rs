//! Item editor stuff.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, io::ClientState, item::container::Storage, show_help_if_needed, tell_user, validate_access};

mod abort;

pub struct IeditCommand;

#[async_trait]
impl Command for IeditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        if ctx.state.is_editing() {
            tell_user!(ctx.writer, "You're already in one or other editor. Finish work there first.\n");
            return;
        }
        show_help_if_needed!(ctx, "iedit");

    }
}