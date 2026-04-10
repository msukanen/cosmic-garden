//! Room editor!

use std::sync::Arc;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, identity::IdentityQuery, io::ClientState, player_or_bust, string::Slugger, tell_user, tell_user_unk, util::access::Accessor};

pub mod abort;

pub struct ReditCommand;

#[async_trait]
impl Command for ReditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        if !plr.read().await.access.is_builder() {
            tell_user_unk!(ctx.writer);
            return;
        }
        let (access, p_id, ploc) = {
            let p = plr.read().await;
            (p.access.clone(), p.id().to_string(), p.location.upgrade())
        };
        if show_help_if_needed(ctx, "redit") { return; }
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
                                ploc.write().await.who.remove(&p_id);
                                target_arc.write().await.who.insert(p_id.clone(), Arc::downgrade(&plr));
                                plr.write().await.location = Arc::downgrade(&target_arc);
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
        {
            let mut w = plr.write().await;
            w.redit_buffer = Some(room);
            ctx.state = ClientState::Editing { player: plr.clone(), mode: EditorMode::Room }
        }

        tell_user!(ctx.writer, "TODO\n")// TODO
    }
}

fn show_help_if_needed(ctx: &mut CommandCtx<'_>, topic: &str) -> bool {
    if !ctx.args.is_empty() && !ctx.args.starts_with('?') {
        return false;
    }

    //TODO invoke help topic rendering
    true
}
