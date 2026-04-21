//! Set 'hardcore' mode on.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, mob::affect::Affect, player_or_bust, show_help, show_help_if_needed, tell_user, thread::life::TICKS_PER_SECOND};

pub struct HardcoreCommand;

#[async_trait]
impl Command for HardcoreCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        if let Some(true) = plr.read().await.hardcore {
            tell_user!(ctx.writer, "Hardcore mode already on.\n");
            return ;
        }

        show_help_if_needed!(ctx, "hardcore");
        if ctx.args.to_lowercase() != "enable" {
            show_help!(ctx, "hardcore");
        }

        let mut p = plr.write().await;
        if p.step_hardcore() {
            drop(p);
            tell_user!(ctx.writer, "<c brown>[<c red>HARDCORE MODE ENABLED!</c>]</c>\n <c blue>*</c> you're permanently PvP-enabled.\n <c blue>*</c> if you die, it's Game Over…\n\n<c yellow>Good luck, brave one!\n");
            return ;
        }
        p.hardcore = Some(false);
        p.affects.insert("HARDCORE".into(), Affect::HardcorePending { remaining: Some((60 * *(TICKS_PER_SECOND.read().await)) as usize) });
        drop(p);
        tell_user!(ctx.writer, "<c brown>[<c yellow>HARDCORE MODE PENDING…</c>] - to enable, retype <c yellow>hardcore enable</c> within 60 seconds.\n");
    }
}
