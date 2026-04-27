//! Room editor!

use std::sync::Arc;

use async_trait::async_trait;

include!(concat!(env!("OUT_DIR"), "/redit_registry.rs"));

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, identity::IdentityQuery, io::ClientState, player::ActivityType, show_help_if_needed, string::Slugger, tell_user, err_tell_user, translocate, util::access::Accessor, validate_access};

pub struct ReditCommand;

#[async_trait]
impl Command for ReditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        if ctx.state.is_editing() {
            err_tell_user!(ctx.writer, "You're already in one or other editor. Finish work there first.\n");
        }
        let (access, p_id, ploc) = {
            let p = plr.read().await;
            (p.access.clone(), p.id().to_string(), p.location.upgrade())
        };
        show_help_if_needed!(ctx, "redit");
        
        // a bit of shortcuts:
        let lc = ctx.args.to_lowercase();
        let room = match lc.as_str() {
            "here" |
            "this" => {
                    let Some(ploc) = ploc else {
                        tell_user!(ctx.writer, "There's absolutely nothing here to edit!\n");
                        return
                    };
                    ploc.read().await.shallow_clone()
                }
            other => {
                // is it even valid ID?
                let Ok(target_id) = other.as_id() else {
                    tell_user!(ctx.writer, "You need to come up with a bit nicer ID…\n");
                    return;
                };
                // see if it's an existing room ID…
                let wr = ctx.world.read().await;
                if let Some(target_arc) = wr.rooms.get(&target_id) {
                    if let Some(ploc) = ploc {
                        if !Arc::ptr_eq(&target_arc, &ploc) {
                            if access.is_true_builder() {
                                translocate!(plr, p_id, ploc, target_arc);
                                tell_user!(ctx.writer, "<c cyan>You phase through the cosmic mists to your destination…</c>\n");
                            } else {
                                tell_user!(ctx.writer, "Unfortunately you have to walk there first… No teleport cheesing with <c red>redit</c>\n");
                                return;
                            }
                        }
                    }
                    target_arc.read().await.shallow_clone()
                } else {
                    tell_user!(ctx.writer, "Mmm no, no such ID exists. How about <c yellow>dig</c>?\n");
                    return;
                }
            }
        };
        // we got a [Room], shove it into buffer and start tinkering.
        {
            let mut w = plr.write().await;
            w.activity_type = ActivityType::Building;
            w.redit_buffer = Some(room);
            ctx.state = ClientState::Editing { player: plr.clone(), mode: EditorMode::Redit{ dirty: false } }
        }
    }
}
