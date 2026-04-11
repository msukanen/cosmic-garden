//! A dummy for dummies.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, tell_user_unk};

pub struct DummyCommand;

#[async_trait]
impl Command for DummyCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        #[cfg(all(debug_assertions, feature = "localtest"))]{
            use crate::tell_user;

            let p = ctx.get_player_arc().unwrap();
            let mut p = p.write().await;
            p.actions_taken += 1;
            tell_user!(ctx.writer, "Actions... {}\n", p.actions_taken);
            return;
        }
        tell_user_unk!(ctx.writer);
    }
}
