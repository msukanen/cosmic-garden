//! Weave buffered item into existence!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, iedit::abort::AbortCommand}, identity::IdentityQuery, io::ClientState, item::{container::Storage, primordial::Metamorphize}, tell_user, validate_access};

pub struct WeaveCommand;

#[async_trait]
impl Command for WeaveCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let (p_id, p_loc) = {
            let p = plr.read().await;
            let p_id = p.id().to_string();
            let Some(p_loc) = p.location.upgrade() else {
                log::error!("Builder '{p_id}' voided? How?! Check their save file!");
                tell_user!(ctx.writer, "Fuuu-uu… Where'd the world go?\nYou're in the void! Nope. No weaving here!\nCheck your save file!\n");
                ctx.state = ClientState::Logout;
                return;
            };
            (p_id, p_loc.clone())
        };

        let item = {
            let mut p = plr.write().await;
            let Some(item) = p.iedit_buffer.take() else {
                drop(p);
                tell_user!(ctx.writer, "Surprisingly enough, there's nothing to weave in your pockets, just dust bunnies.\n");
                AbortCommand.exec({ctx.args = "q";ctx}).await;
                return;
            };
            item
        };

        let final_item = item.metamorph();
        log::trace!("Builder metamorph: {final_item:?}");
        let Err(storage_error) = plr.write().await.inventory.try_insert(final_item) else {
            tell_user!(ctx.writer, "You successfully created something - check your inventory…\n");
            AbortCommand.exec({ctx.args = "q";ctx}).await;
            return;
        };
        // trusting the system...
        let item = storage_error.extract_item().unwrap();
        // too big for player inv. Room?
        let Err(storage_error) = p_loc.write().await.contents.try_insert(item) else {
            tell_user!(ctx.writer, "Well, too large for your inventory. So, you set it down on the ground…\n");
            AbortCommand.exec({ctx.args = "q";ctx}).await;
            return;
        };
        // trusting the system again...
        let item = storage_error.extract_item().unwrap();
        // TODO couldn't fit in room - lost-and-found
        tell_user!(ctx.writer, "Well… you made something, but have no clue where it went…\n");
        AbortCommand.exec({ctx.args = "q";ctx}).await;
    }
}
