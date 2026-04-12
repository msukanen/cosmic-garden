//! Item editor stuff.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, edit::EditorMode, identity::IdentityQuery, io::ClientState, item::{container::Storage, primordial::PrimordialItem}, show_help_if_needed, tell_user, validate_access};

pub mod abort;
pub mod desc;
pub mod devolve;
pub mod iex;
pub mod set;
pub mod weave;

#[macro_export]
macro_rules! err_iedit_buffer_inaccessible {
    ($ctx:ident, $p:ident, $p_id:ident) => {
        drop($p);
        log::error!("Builder '{}'.iedit_buffer evaporated mid-edit?!", $p_id);
        crate::tell_user!($ctx.writer, "Uh-oh, editor buffer evaporated?!\n");
        return;
    };
}

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

        match &target_item {
            Some(i) => tell_user!(ctx.writer, "You found '{}', let see about what it's made of…\n", i.id()),
            _ => tell_user!(ctx.writer, "No such item was found. So, lets create new stuff!\n")
        }
        {   // tuck it into safety of iedit_buffer which will be stored on disk the moment io_thread sez so.
            let mut w = plr.write().await;
            w.iedit_buffer = target_item.or_else(|| Some(PrimordialItem::new(id)));
        }
        ctx.state = ClientState::Editing { player: plr.clone(), mode: EditorMode::Item { dirty: true } };
    }
}