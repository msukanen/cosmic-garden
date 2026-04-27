//! MEdit – a.k.a. "mob editor".

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, err_tell_user, identity::IdentityQuery, io::ClientState, mob::core::Entity, player::ActivityType, roomloc_or_bust, show_help_if_needed, tell_user, thread::librarian::ENT_BP_LIBRARY, validate_access};

include!(concat!(env!("OUT_DIR"), "/medit_registry.rs"));

pub struct MeditCommand;

#[async_trait]
impl Command for MeditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, true_builder);
        if ctx.state.is_editing() {
            err_tell_user!(ctx.writer, "You're already in one or other editor. Finish work there first.\n");
        }
        let loc = roomloc_or_bust!(plr);
        show_help_if_needed!(ctx, "medit");

        let mut found: Option<Entity> = None;
        // try find with direct ID in room
        if let Some(ent) = loc.read().await.entities.get(ctx.args) {
            found = ent.read().await.shallow_clone().into();
        }
        // …or world, if not in room…
        else if let Some(ent) = ctx.world.read().await.entities.get(ctx.args) {
            if let Some(arc) = ent.upgrade() {
                found = arc.read().await.shallow_clone().into();
            }
        }
        // wasn't found yet. Get a blueprint?
        else {
            if let Some(bp) = ENT_BP_LIBRARY.read().await.get(ctx.args) {
                found = bp.into();
            }
        }

        if let Some(found) = found {
            let mut p = plr.write().await;
            p.activity_type = ActivityType::Building;
            tell_user!(ctx.writer, "MEdit invoked for <c cyan>{}</c> #<c gray>'{}'</c>", found.id(), found.title());
            p.medit_buffer = found.into();
            ctx.state = ClientState::Editing { player: plr.clone(), mode: EditorMode::Medit { dirty: false } };
            return ;
        }

        tell_user!(ctx.writer, "No such entity or blueprint as '{}'…\n", ctx.args);
    }
}
