//! Weave the words into persistent form.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, cmd_alias::BufferNuke}, thread::librarian::HELP_LIBRARY, tell_user, validate_access, validate_editor_mode};

pub struct WeaveCommand;

#[async_trait]
impl Command for WeaveCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        validate_editor_mode!(ctx, "HEdit");
        if !ctx.state.is_dirty() {
            tell_user!(ctx.writer, "Not dirty... *stretch* - done with edits?\n");
            BufferNuke.exec({ctx.args = "quiet";ctx}).await;
            return;
        }

        let mut p = plr.write().await;
        let page = p.hedit_buffer.take();
        if page.is_none() {
            log::error!("WHERE DID THE PAGE GO?!");
        }
        drop(p);

        if let Some(page) = page {
            if !(*HELP_LIBRARY).write().await.shelve(&page, &ctx.out) {
                tell_user!(ctx.writer, "Something's off… Probably need to check your work closer.\nEither the librarian is busy or there's something else wrong…\n");
                let mut p = plr.write().await;
                p.hedit_buffer = Some(page);
                return ;
            }
            tell_user!(ctx.writer, "*You dust your hands and leave the librarian to do his work.*\n");
        }

        tell_user!(ctx.writer, "*stretch* - done with edits?\n");
        BufferNuke.exec({ctx.args = "quiet";ctx}).await;
    }
}
