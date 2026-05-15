//! Reload e.g. a specified room.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, roomloc_or_bust, show_help_if_needed, thread::SystemSignal, validate_access};

pub struct ReloadCommand;

#[async_trait]
impl Command for ReloadCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, true_builder);
        show_help_if_needed!(ctx, "reload");

        match ctx.args {
            "here"|"this" => {
                let r = roomloc_or_bust!(plr);
                ctx.out.janitor.send(SystemSignal::ReloadRoom { arc: r.clone() }).ok();
            }

            r_id => {
                if let Some(r) = ctx.world.read().await.get_room_by_id(r_id) {
                    ctx.out.janitor.send(SystemSignal::ReloadRoom { arc: r.clone() }).ok();
                } else {
                    err_tell_user!(ctx.writer, "Nope, no such place as '{}' in the maps…\n", r_id);
                }
            }
        }
    }
}
