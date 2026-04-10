//! Staff-only invisibility.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, player_or_bust, tell_user, tell_user_unk, util::access::Accessor};

pub struct InvisCommand;

#[async_trait]
impl Command for InvisCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        if !plr.read().await.access.is_true_builder() {
            tell_user_unk!(ctx.writer);
            return;
        }
        
        let mut cfg = plr.read().await.config.clone();
        cfg.is_ghost = match if ctx.args.is_empty() {
            // flip if no args
            if cfg.is_ghost {'0'} else {'1'}
        } else {
            ctx.args.chars().nth(0).unwrap()
        } {
            '0'|'f'|'n' => false,
            _ => true
        };
        tell_user!(ctx.writer, "You {}.\n", match cfg.is_ghost {
            true => "are a ghost now! Woo, spooky!",
            _ => "are visible"
        });
        plr.write().await.config.is_ghost = cfg.is_ghost;
    }
}
