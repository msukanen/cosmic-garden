//! Quitter!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, io::{Broadcast, ClientState}, player_or_bust};

pub struct QuitCommand;

#[async_trait]
impl Command for QuitCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        ctx.state = ClientState::Logout;// regardless if they're in the world (yet), Logout.
        let plr = player_or_bust!(ctx);
        let (room, p_title) = {
            let p = plr.read().await;
            (p.location.upgrade(), p.title().to_string())
        };
        // in vicinity, notify anyone interested that X has gone the way of Dodo.
        if let Some(r) = room {
            ctx.out.broadcast.send(Broadcast::Logout {
                from: r,
                who: p_title,
            }).ok();
        }
    }
}
