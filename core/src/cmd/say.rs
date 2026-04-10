//! Say something!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, io::Broadcast, player_or_bust, tell_user};

pub struct SayCommand;

#[async_trait]
impl Command for SayCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        if ctx.args.is_empty() {
            tell_user!(ctx.writer, "Silence is golden, they say.\n");
            return;
        }

        let room = {
            let p = plr.read().await;
            let Some(room) = p.location.upgrade() else {
                tell_user!(ctx.writer, "In the void no one can hear you…\n");
                return;
            };
            room.clone()
        };

        let message = ctx.args.to_string();
        tell_user!(ctx.writer, "You say \"{}\"\n", message);
        ctx.tx.send(Broadcast::Say {
            room,
            message,
            from: plr.clone(),
        }).ok();
    }
}
