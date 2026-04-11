//! Weave buffered item into existence!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, iedit::abort::AbortCommand}, identity::IdentityQuery, io::ClientState, item::{container::Storage, primordial::Metamorphize}, tell_user, validate_access};

pub struct WeaveCommand;

macro_rules! bye_weave {
    ($ctx:ident, $plr:ident) => {
        {
            let mut p = $plr.write().await;
            p.iedit_buffer = None;
        }
        $ctx.state = ClientState::Playing { player: $plr.clone() };
        return;
    };
}

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

        let mut p = plr.write().await;
        let Some(item) = p.iedit_buffer.take() else {
            tell_user!(ctx.writer, "Surprisingly enough, there's nothing to weave in your pockets, just dust bunnies.\n");
            AbortCommand.exec({ctx.args = "q";ctx}).await;
            return;
        };

        let final_item = item.metamorph();
        if let Err(uhoh) = p.inventory.try_insert(final_item) {
            tell_user!(ctx.writer, "No space in your inventory! Dropping into room, maybe…\n");
            drop(p);
            let mut r = p_loc.write().await;
            if let Some(item) = uhoh.extract_item() {
                if let Err(uhoh) = r.contents.try_insert(item) {
                    drop(r);
                    log::error!("Item in {uhoh:?} too large to be contained by even a Room! Eww!");
                    tell_user!(ctx.writer, "Yikes?! Room cannot hold on to that? What's this buggery?\n");
                    // TODO: lost-and-found mechanics for io_thread
                    //     - store such items on World level?
                    bye_weave!(ctx,plr);
                }
                tell_user!(ctx.writer, "Well, too large for your inventory. So, you set it down on the ground…\n");
                bye_weave!(ctx,plr);
            }
            log::error!("Item evaporated during transit?! DEV! FIX! StorageError *should* have had it!");
            tell_user!(ctx.writer, "Uh, where'd it go? Probably should check the logs…\n");
        };

        bye_weave!(ctx,plr);
    }
}
