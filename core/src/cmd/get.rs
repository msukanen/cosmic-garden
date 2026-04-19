//! Get something from ground…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, item::container::Storage, player_or_bust, roomloc_or_bust, show_help_if_needed, tell_user, thread::io::add_item_to_lnf, util::activity::ActionWeight};

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
        let act_w = item.required_space();
        let Err(item_err) = plr.write().await.inventory.try_insert(item) else {
            tell_user!(ctx.writer, "You nab '{}'!\n", item_name);
            plr.write().await.act(plr.clone(),  &ctx.system, ActionWeight::ItemTransfer { count: act_w as usize }).await;
            return;
        };
        // bugger, no space in inventory, lets put it back...
        let Err(item_err) = p_loc.write().await.contents.try_insert(item_err.into()) else {
            tell_user!(ctx.writer, "Way too big or heavy. You set it back before you break your back.\n");
            return;
        };
        add_item_to_lnf(item_err).await;
        tell_user!(ctx.writer, "… the world is being weird …\n");
    }
}
