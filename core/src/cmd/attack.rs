//! Attack something, maybe.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, player_or_bust, roomloc_or_bust, show_help_if_needed, string::StrUuid, tell_user, thread::SystemSignal};

pub struct AttackCommand;

#[async_trait]
impl Command for AttackCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        show_help_if_needed!(ctx, "attack");
        let loc = roomloc_or_bust!(plr);
        {
            let r = loc.read().await;
            for ent_id in r.entities.keys() {
                if ent_id.show_uuid(false).starts_with(ctx.args) {
                    ctx.out.life.send(SystemSignal::Attack { who: plr.clone(), victim_id: ent_id.into() }).ok();
                    return ;
                }
            }
        }
        tell_user!(ctx.writer, "You squint hard, but no such thing seems to be here…\n");
    }
}
