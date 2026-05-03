//! Item editor stuff.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, identity::IdentityQuery, io::ClientState, item::{container::storage::Storage, primordial::PrimordialItem}, player::ActivityType, roomloc_or_bust, show_help_if_needed, tell_user, thread::librarian::get_item_blueprint, validate_access};

include!(concat!(env!("OUT_DIR"), "/iedit_registry.rs"));

pub struct IeditCommand;

/// IEdit
// 
// usage: iedit <item>
#[async_trait]
impl Command for IeditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let p_loc = roomloc_or_bust!(plr);
        
        if ctx.state.is_editing() {
            tell_user!(ctx.writer, "You're already in one or other editor. Finish work there first.\n");
            return;
        }
        show_help_if_needed!(ctx, "iedit");
        
        let id = ctx.args;
        let target_item = {
            let mut p = plr.write().await;
            let found = p.inventory.take_by_name(id);
            drop(p);
            // was in inventory already or not?
            if found.is_none() {
                // room maybe?
                if let Some(item) = p_loc.write().await.take_by_name(id) {
                    Some(item)
                } else {
                    get_item_blueprint(id, &ctx.out).await
                }
            } else {
                found
            }
        };

        match &target_item {
            Some(i) => tell_user!(ctx.writer, "You found '{}', let see about what it's made of…\n", i.id()),
            _ => tell_user!(ctx.writer, "No such item was found. So, lets create new stuff!\n")
        }
        {   // tuck it into safety of iedit_buffer which will be stored on disk the moment io_thread sez so.
            let mut w = plr.write().await;
            w.iedit_buffer = target_item.or_else(|| Some(PrimordialItem::new(id)));
            w.activity_type = ActivityType::Building;
        }
        ctx.state = ClientState::Editing { player: plr.clone(), mode: EditorMode::Iedit { dirty: true } };
    }
}