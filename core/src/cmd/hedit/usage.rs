//! Modify "usage" field of a [HelpPage].

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, tell_user, validate_editor_mode, util::ed::{EdResult, edit_text}, validate_access};

pub struct UsageCommand;

#[async_trait]
impl Command for UsageCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        validate_editor_mode!(ctx, "HEdit");
        let mut doc = {
            let p = plr.read().await;
            let Some(buffer) = &p.hedit_buffer else {
                log::error!("Builder '{}' lost their hedit_buffer!", p.id());
                tell_user!(ctx.writer, "Remarkable… You were editing something, but it vanished?!\n");
                return ;
            };
            buffer.clone()
        };
        if ctx.args.is_empty() {
            tell_user!(ctx.writer, "<c green>---[<c yellow>{}</c>]---</c>\n{}<c red>// END</c>", doc.id(), doc.usage());
            return ;
        }

        let mut source = doc.usage.join("\n");
        source.push('\n');

        match edit_text(ctx.writer, ctx.args, &source).await {
            Ok(EdResult::ContentReady { text, dirty, verbose }) => {
                if dirty {
                    doc.usage = text.lines()
                        .map(|s| s.trim_end().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    plr.write().await.hedit_buffer = Some(doc);
                    if !verbose {
                        tell_user!(ctx.writer, "Syntax rules updated.\n");
                    }
                    return ;
                }},

            Ok(EdResult::NoChanges(_)) => tell_user!(ctx.writer, "No changes? Okays then.\n"),
            _ => ()
        }
    }
}

#[cfg(test)]
mod hedit_usage_tests {
    use std::io::Cursor;

    use crate::{cmd::{hedit::{HeditCommand, usage::UsageCommand}, help::HelpCommand}, ctx, get_operational_mock_librarian, stabilize_threads, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn dummy_entry_usage_check() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let _ = get_operational_mock_librarian!(c,w);
        stabilize_threads!();
        let state = ctx!(state, HeditCommand, "dummy",s,c.out,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, HeditCommand, "dummy",s,c.out,w,|out:&str| out.contains("no such topic"));
        let state = ctx!(state, HeditCommand, "new dummy",s,c.out,w,|out:&str| out.contains("DUMMY"));
        let state = ctx!(state, UsageCommand, "",s,c.out,w,|out:&str| out.contains("Usage:"));
        let state = ctx!(state, UsageCommand, "dummy <stuff>",s,c.out,w,|out:&str| out.contains("updated"));
        let state = ctx!(state, UsageCommand, "dummy bar <foo> <baz>",s,c.out,w,|out:&str| out.contains("updated"));
        let state = ctx!(state, UsageCommand, "",s,c.out,w,|out:&str| out.contains("Usage:") && out.contains("<stuff>") && out.contains("<baz>"));
        let state = ctx!(state, UsageCommand, "-1",s,c.out,w,|out:&str| !out.contains("<stuff>"));
        let state = ctx!(state, UsageCommand, "+7 dummy baz <bar>",s,c.out,w);
        let state = ctx!(state, UsageCommand, "",s,c.out,w,|out:&str| out.contains("Usage:") && out.contains("<foo>") && out.contains("<bar>"));
        let _ = ctx!(state, HelpCommand, "",s,c.out,w);
    }
}
