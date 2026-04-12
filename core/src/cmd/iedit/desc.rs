use async_trait::async_trait;
use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, player_or_bust, show_help, string::{Describable, DescribableMut}, tell_user, util::ed::{EdResult, edit_text}};

pub struct DescCommand;

#[async_trait]
impl Command for DescCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let p_id = {
            let p = plr.read().await;
            let p_id = p.id().to_string();
            p_id
        };
        let res = edit_text(ctx.writer, ctx.args,
            &{
                let mut p = plr.write().await;
                let Some(buf) = p.iedit_buffer.as_mut() else {
                    log::error!("Builder '{p_id}' lost their beanie. As in, their iedit_buffer evaporated mid-edit?!");
                    tell_user!(ctx.writer, "Aw shucks, editor contents poofed?!\n");
                return;
            }; buf.desc().to_string()}
        ).await;
        let verbose = match res {
            Ok(EdResult::ContentReady { text, verbose, .. }) => {
                let Some(ref mut b) = plr.write().await.iedit_buffer else {
                    log::error!("Whatever happened to Iedit buffer here...?");
                    return ;
                };
                b.set_desc(&text);
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
