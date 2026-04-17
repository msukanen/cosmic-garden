//! Help system basics…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, player_or_bust, string::Describable, tell_user, thread::librarian::HELP_LIBRARY, util::{HelpPage, access::Accessor}};

pub struct HelpCommand;

#[async_trait]
impl Command for HelpCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        // check if a builder and if they have an active buffer they want to preview…
        let access = {
            let p = plr.read().await;
            if p.access.is_builder() && ctx.args.is_empty() {
                if let Some(ref page) = p.hedit_buffer {
                    RenderHelpPage.render(ctx, page, true).await;
                    return ;
                }
            }
            p.access.clone()
        };

        if ctx.args.is_empty() {
            HelpCommand.exec({ctx.args = "help"; ctx}).await;
            return ;
        }

        // .get() doing a check is probably redundant, or .render() doing it is redundant, but whatever XD … double or nothing.
        if let Some(page) = &(*HELP_LIBRARY).read().await.get(ctx.args, &access.into(), false) {
            if RenderHelpPage.render(ctx, page, false).await {
                return ;
            }
        }

        tell_user!(ctx.writer, "Unfortunately, nothing about such a topic seems to at hand…\n");
    }
}

struct RenderHelpPage;
/// Helper trait to get things done with a help page entry...
#[async_trait]
trait HelpRenderCmd: Send + Sync { async fn render(&self, ctx: &mut CommandCtx<'_>, page: &HelpPage, bypass_access: bool ) -> bool; }
#[async_trait]
impl HelpRenderCmd for RenderHelpPage {
    /// Render a [HelpPage].
    /// 
    /// # Args
    /// - `ctx`
    /// - `page` to render
    /// - `bypass_access`, if 'true', will render the page to anyone. Use sparingly…
    async fn render(&self, ctx: &mut CommandCtx<'_>, page: &HelpPage, bypass_access: bool) -> bool {
        // let's see if we're allowed to even view the page…?
        if !bypass_access && !page.can_access(&ctx.get_player_arc().unwrap().read().await.access) {
            return false;// well shucks... no rights to read. Oh well!
        }
        
        let header = format!(   "<c yellow>--- [ Help: {} ] ---</c>\n", page.title().to_uppercase());
        let body = &page.desc();
        let mut alias_div = String::new();
        if !page.alias.is_empty() {
            alias_div.push_str(&format!("<c gray>   -→ also: {}</c>\n", page.alias.iter().map(|s|s.as_str()).collect::<Vec<_>>().join(", ")));
        }
        let usage = &page.usage();
    
        // …and render…
        tell_user!(ctx.writer, "{}{}\n{}\n\n{}", header, alias_div, body, usage);
        true
    }
}
