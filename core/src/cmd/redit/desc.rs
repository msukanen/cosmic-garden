//! Descriptions for all!
use async_trait::async_trait;
use crate::{access_ed_entry, cmd::{Command, CommandCtx}, validate_access, show_help, util::ed::{EdResult, edit_text}};

pub struct DescCommand;

#[async_trait]
impl Command for DescCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let res = edit_text(ctx.writer, ctx.args, &access_ed_entry!(plr, redit_buffer).desc).await;
        let verbose = match res {
            Ok(EdResult::ContentReady { text, verbose, .. }) => {
                let Some(ref mut b) = plr.write().await.redit_buffer else {
                    log::error!("Whatever happened to REdit buffer here...?");
                    return ;
                };
                b.desc = text;
                ctx.state.set_dirty(true);
                verbose
            },
            Ok(EdResult::NoChanges(true)) => true,
            Ok(EdResult::HelpRequested) => {
                show_help!(ctx, "edit-desc");
            },
            _ => false
        };
        
        if verbose {// re-run argless to pretty-print current description.
            let cmd = DescCommand;
            cmd.exec({ctx.args = ""; ctx}).await;
        }

        if ctx.args.starts_with('?') {
            show_help!(ctx, "edit-desc");
        }
    }
}
