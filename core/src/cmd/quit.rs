//! Quitter!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, io::{Broadcast, ClientState}, player_or_bust};

pub struct QuitCommand;

#[async_trait]
impl Command for QuitCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let (room, p_title) = {
            let p = plr.read().await;
            (p.location.upgrade(), p.title().to_string())
        };
        if let Some(r) = room {
            ctx.tx.send(Broadcast::Logout {
                from: r,
                who: p_title,
            }).ok();
        }
        ctx.state = ClientState::Logout
    }
}
