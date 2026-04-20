//! Goto somewhere, somehow

use std::sync::Arc;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, look::LookCommand}, identity::IdentityQuery, io::Broadcast, player::Player, tell_user, thread::SystemSignal, util::direction::Direction};

pub struct GotoCommand;

#[async_trait]
impl Command for GotoCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let args = ctx.args.trim();
        if args.is_empty() {
            tell_user!(ctx.writer, "Going really fast nowhere…?\n");
            return ;
        }

        let dir = Direction::from(args);

        let Some(plr) = ctx.get_player_arc() else {
            tell_user!(ctx.writer, "Something weird is going on…\n");
            log::error!("Where'd the Player go…?");
            return
        };

        let Some(origin) = plr.read().await.location.upgrade() else {
            let (p_id, p_name) = {
                let lock = plr.read().await;
                (lock.id().to_string(), lock.name.clone())
            };
            log::error!("Player '{p_id}'/'{p_name}' in void! Go rescue!");
            tell_user!(ctx.writer, "You are floating in the void…\n");
            return
        };

        let target_arc = {
            let r_lock = origin.read().await;
            r_lock.exits.get(&dir).and_then(|e| e.upgrade())
        };

        let Some(target) = target_arc else {
            tell_user!(ctx.writer, "Alas, you have no means to go {}.\n", dir);
            return
        };

        log::debug!("Requesting transport via {dir} to {}", target.read().await.id());
        if let Err(e) = ctx.out.life.send(
                SystemSignal::WantTransportFromTo {
                    who: plr.clone(),
                    from: origin.clone(),
                    to: target.clone(),
                    via: dir
                }
        ) {
            log::warn!("Transport system clogged… {e:?}");
            tell_user!(ctx.writer, "You trip briefly, losing your orientation…\n");
            return;
        }
    }
}
