//! Drop the mic!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, thread::add_item_to_lnf, item::container::Storage, player_or_bust, roomloc_or_bust, tell_user, util::activity::ActionWeight};

pub struct DropCommand;

#[async_trait]
impl Command for DropCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let p_loc = roomloc_or_bust!(plr);
        if ctx.args.is_empty() {
            tell_user!(ctx.writer, "You drop the mic. Well, actually no - what do you want to drop?\n");
            return;
        }
        let what = ctx.args;
        let thing_id = {
            let p = plr.read().await;
            let Some(id) = p.inventory.find_id_by_name(what) else {
                tell_user!(ctx.writer, "You don't seem to be carrying any '{}'.\n", what);
                return;
            };
            id
        };
        let Some(item) = plr.write().await.inventory.take(&thing_id) else {
            tell_user!(ctx.writer, "Wait, it was here a second ago… Where'd it go?\n");
            return;
        };
        let mut r = p_loc.write().await;
        let item_name = item.title().to_string();
        if let Err(storage_error) = r.try_insert(item) {
            drop(r);
            let mut p = plr.write().await;
            // if the try_insert fails - something's really wrong...
            if let Err(e) = p.inventory.try_insert(storage_error.into()) {
                add_item_to_lnf(e).await;
                tell_user!(ctx.writer, "You dropped it, and it instantly vanished? Huh?!\n");
            } else {
                tell_user!(ctx.writer, "The room is too cluttered! You can't find a place for {}.\n", item_name);
            }
            return;
        }

        tell_user!(ctx.writer, "You drop '{}' on the ground.\n", item_name);

        plr.write().await.act(plr.clone(),  &ctx.out, ActionWeight::ItemTransfer { count: 1 }).await;
    }
}
