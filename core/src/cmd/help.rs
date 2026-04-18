//! Help system basics…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, identity::IdentityQuery, player_or_bust, string::Describable, tell_user, thread::librarian::HELP_LIBRARY, util::{HelpPage, access::Accessor}};

pub struct HelpCommand;

#[async_trait]
impl Command for HelpCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        // check if a) args is empty, b) builder and if they have c) an active buffer they want to preview…
        let access = {
            let p = plr.read().await;
            if p.access.is_builder() && ctx.args.is_empty() {
                if let Some(ref page) = p.hedit_buffer {
                    RenderHelpPage.render(ctx, RenderHelpMode::Full, page, true).await;
                    return ;
                }
            }
            p.access.clone()
        };

        if ctx.args.is_empty() {
            HelpCommand.exec({ctx.args = "help"; ctx}).await;
            return ;
        }

        let (mode, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        let mode = match mode {
            "u" => { ctx.args = args; RenderHelpMode::UsageOnly },
            "q" => { ctx.args = args; RenderHelpMode::Quick },
            _ => RenderHelpMode::default()
        };

        let page_id = match ctx.state.edit_mode() {
            Some(EditorMode::Help { .. }) => format!("hedit:{}", ctx.args),
            Some(EditorMode::Room { .. }) => format!("redit:{}", ctx.args),
            Some(EditorMode::Item { .. }) => format!("iedit:{}", ctx.args),
            None => ctx.args.to_string()
        };

        // .get() doing a check is probably redundant, or .render() doing it is redundant, but whatever XD … double or nothing.
        if let Some(page) = &(*HELP_LIBRARY).read().await.get(&page_id, &access.into(), false) {
            if RenderHelpPage.render(ctx, mode, page, false).await {
                return ;
            }
        }

        tell_user!(ctx.writer, "Unfortunately, nothing about such a topic seems to at hand…\n");
    }
}

enum RenderHelpMode {
    Full,
    Quick,
    UsageOnly,
}

impl Default for RenderHelpMode {
    fn default() -> Self {
        Self::Full
    }
}

struct RenderHelpPage;
/// Helper trait to get things done with a help page entry...
#[async_trait]
trait HelpRenderCmd: Send + Sync { async fn render(&self, ctx: &mut CommandCtx<'_>, mode: RenderHelpMode, page: &HelpPage, bypass_access: bool ) -> bool; }
#[async_trait]
impl HelpRenderCmd for RenderHelpPage {
    /// Render a [HelpPage].
    /// 
    /// # Args
    /// - `ctx`
    /// - `page` to render
    /// - `bypass_access`, if 'true', will render the page to anyone. Use sparingly…
    async fn render(&self, ctx: &mut CommandCtx<'_>, mode: RenderHelpMode, page: &HelpPage, bypass_access: bool) -> bool {
        // let's see if we're allowed to even view the page…?
        if !bypass_access && !page.can_access(&ctx.get_player_arc().unwrap().read().await.access) {
            return false;// well shucks... no rights to read. Oh well!
        }

        let (header, body, alias_div, usage) = match mode {
            RenderHelpMode::Full => {
                let header = format!(   "<c yellow>--- [ Help: {} ] ---</c>\n", page.title().to_uppercase());
                let body = page.desc();
                let mut alias_div = String::new();
                if !page.alias.is_empty() {
                    alias_div.push_str(&format!("<c gray>   -→ also: {}</c>\n", page.alias.iter().map(|s|s.as_str()).collect::<Vec<_>>().join(", ")));
                }
                let usage = page.usage();
                (header, body, alias_div, usage)
            },

            RenderHelpMode::Quick => ("".into(), page.desc(), "".into(), page.usage()),
            RenderHelpMode::UsageOnly => ("".into(), "", "".into(), page.usage())
        };
        
        // …and render…
        match mode {
            RenderHelpMode::Full => tell_user!(ctx.writer, "{}{}\n{}\n\n{}", header, alias_div, body, usage),
            RenderHelpMode::Quick => tell_user!(ctx.writer, "{}\n\n{}", body, usage),
            RenderHelpMode::UsageOnly => tell_user!(ctx.writer, "{}", usage)
        }
        true
    }
}

#[cfg(test)]
mod cmd_help_tests {
    use std::{io::Cursor, time::Duration};
    use tokio::sync::mpsc;

    use super::*;
    use crate::{cmd::{hedit::{HeditCommand, abort::AbortCommand, desc::DescCommand, weave::WeaveCommand}, iedit::IeditCommand}, ctx, io::ClientState, thread::{SystemSignal, librarian::librarian}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn namespacing_get() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,tx,mut ch,p) = get_operational_mock_world().await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, HelpCommand, "iedit:set",s,tx,ch,w,p,|out:&str| out.contains("nothing about"));
        let state = ctx!(state, HeditCommand, "iedit:set",s,tx,ch,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, HeditCommand, "iedit:set",s,tx,ch,w,p,|out:&str| out.contains("hedit new"));
        let state = ctx!(state, HeditCommand, "new iedit:set",s,tx,ch,w,p,|out:&str| out.contains("desc ="));
        let state = ctx!(state, HelpCommand, "iedit:set",s,tx,ch,w,p,|out:&str| out.contains("nothing about"));
        let state = ctx!(state, HelpCommand, "",s,tx,ch,w,p,|out:&str| out.contains("desc ="));
        // as librarian is not running by default... fire a mock one up
        let lrx = mpsc::channel::<SystemSignal>(8);
        ch.librarian_tx = lrx.0;
        tokio::spawn(librarian((ch.clone(), lrx.1)));
        tokio::time::sleep(Duration::from_secs(2)).await;// let the lib stabilize...
        let state = ctx!(state, DescCommand, "= New stuff?",s,tx,ch,w,p);
        let state = ctx!(state, WeaveCommand, "",s,tx,ch,w,p);
        tokio::time::sleep(Duration::from_secs(2)).await;
        let state = ctx!(state, HelpCommand, "iedit:set",s,tx,ch,w,p,|out:&str| out.contains("New stuff?\n\n"));
        let state = ctx!(state, HelpCommand, "iedit-set",s,tx,ch,w,p,|out:&str| out.contains("New stuff?\n\n"));
        let state = ctx!(state, HeditCommand, "new dummy",s,tx,ch,w,p,|out:&str| out.contains("desc ="));
        let state = ctx!(state, HelpCommand, "set",s,tx,ch,w,p,|out:&str| out.contains("nothing about"));
        let state = ctx!(state, AbortCommand, "",s,tx,ch,w,p);
        let state = ctx!(state, IeditCommand, "apple",s,tx,ch,w,p);
        let state = ctx!(state, HelpCommand, "set",s,tx,ch,w,p,|out:&str| out.contains("New stuff?\n\n"));
    }
}
