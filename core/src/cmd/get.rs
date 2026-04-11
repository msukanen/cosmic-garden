//! Get something from ground…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, item::container::Storage, player_or_bust, roomloc_or_bust, show_help_if_needed, tell_user};

pub struct GetCommand;

#[async_trait]
impl Command for GetCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        show_help_if_needed!(ctx, "get");
        let p_loc = roomloc_or_bust!(plr);

        let what = ctx.args;
        let thing_id = {
            let r = p_loc.read().await;
            let Some(thing) = r.contents.find_id_by_name(what) else {
                tell_user!(ctx.writer, "No such thing here…\n");
                return;
            };
            thing
        };
        let Some(item) = p_loc.write().await.contents.take(&thing_id) else {
            tell_user!(ctx.writer, "It's stuck?\n");
            return;
        };
        let item_name = item.title().to_string();
        let Err(storage_error) = plr.write().await.inventory.try_insert(item) else {
            tell_user!(ctx.writer, "You nab '{}'!\n", item_name);
            return;
        };
        // bugger, no space in inventory, lets put it back...
        let Some(item) = storage_error.extract_item() else {
            log::error!("How can matter evaporate. Dig the logs!");
            tell_user!(ctx.writer, "There's something seriously wrong in timespace continuum…\n");
            return;
        };
        let Err(storage_error) = p_loc.write().await.contents.try_insert(item) else {
            tell_user!(ctx.writer, "Way too big or heavy. You set it back before you break your back.\n");
            return;
        };
        let r_id = p_loc.read().await.id().to_string();
        log::error!("Item '{thing_id}' belonged to room '{r_id}', but it can't be put back. Why? {storage_error:?}\n");
        // TODO fire up lost and found!
        tell_user!(ctx.writer, "... the world is being weird ...\n");
    }
}
