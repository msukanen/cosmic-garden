//! Help editor!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, help::HelpCommand}, player::ActivityType, player_or_bust, show_help_if_needed, tell_user, thread::librarian::get_help_page, util::HelpPage, validate_access};

// Get modules.
include!(concat!(env!("OUT_DIR"), "/hedit_registry.rs"));

pub struct HeditCommand;

#[async_trait]
impl Command for HeditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        if ctx.state.is_editing() {
            tell_user!(ctx.writer, "You're already in one or other editor. Finish work there first.\n");
            return;
        }
        show_help_if_needed!(ctx, "hedit");
        let access = plr.read().await.access.clone();

        let (intent, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));

        // some of the "intents" auto-block creation of a topic, ofc., like "new".
        match intent.to_lowercase().as_str() {
            "new" => {
                if args.is_empty() {
                    tell_user!(ctx.writer, "A new topic? Sure, but about what?\n");
                    return ;
                }

                if let Some(page) = get_help_page(args, access.clone(), false, &ctx.out).await {
                    Edit.edit(ctx, page, false).await;
                    return;
                } else {
                    // a brand new page! Lets see if the help system lets us make it…
                    if let Ok(page) = HelpPage::new(args) {
                        Edit.edit(ctx, page, true).await;
                        return;
                    }
                    
                    tell_user!(ctx.writer, "Uhm, you probably would like to rethink the topic name…\n");
                    return;
                }
            }
            _ => {
                if let Some(page) = get_help_page(ctx.args, access.clone(), false, &ctx.out).await {
                    // old page, lets work on that.
                    Edit.edit(ctx, page, false).await;
                    return;
                }
            }
        }

        tell_user!(ctx.writer, "There is no such topic as '{}', yet…\nIf you really want to create it, use <c yellow>hedit new <topic></c>.\n", intent);
    }
}

#[async_trait]
trait HeditEdit {async fn edit(&self, ctx: &mut CommandCtx<'_>, page: HelpPage, new: bool);}
struct Edit;
#[async_trait]
impl HeditEdit for Edit {
    async fn edit(&self, ctx: &mut CommandCtx<'_>, page: HelpPage, new: bool) {
        let plr = player_or_bust!(ctx);
        ctx.state = crate::io::ClientState::Editing { player: plr.clone(), mode: crate::edit::EditorMode::Hedit { dirty: new } };
        let mut p = plr.write().await;
        p.hedit_buffer = Some(page);
        p.activity_type = ActivityType::Building;
        drop(p);
        HelpCommand.exec({ctx.args = "";ctx}).await;
    }
}
