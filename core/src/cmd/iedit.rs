//! Item editor stuff.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, io::ClientState, item::container::Storage, player, show_help_if_needed, tell_user, validate_access};

pub mod abort;

pub struct IeditCommand;

#[async_trait]
impl Command for IeditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        if ctx.state.is_editing() {
            tell_user!(ctx.writer, "You're already in one or other editor. Finish work there first.\n");
            return;
        }
        show_help_if_needed!(ctx, "iedit");
        
        let id = ctx.args;
        let target_item = {
            let mut p = plr.write().await;
            let p_lock = p.location.upgrade();
            let found = p.inventory.take(id);
            drop(p);
            if found.is_none() {
                if let Some(arc) = p_lock {
                    arc.write().await.contents.take(id)
                } else {
                    None
                }
            } else {
                found
            }
        };

        if target_item.is_some() {
            {   // tuck it into safety of iedit_buffer which will be stored on disk the moment io_thread sez so.
                let mut w = plr.write().await;
                w.iedit_buffer = target_item;
            }
            ctx.state = ClientState::Editing { player: plr.clone(), mode: EditorMode::Item { dirty: true } };
        }
    }
}