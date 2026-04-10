//! Goto somewhere, somehow

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, look::LookCommand}, identity::IdentityQuery, io::Broadcast, player::Player, tell_user, util::direction::Direction};

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

        let Some(room) = plr.read().await.location.upgrade() else {
            let (p_id, p_name) = {
                let lock = plr.read().await;
                (lock.id().to_string(), lock.name.clone())
            };
            log::error!("Player '{p_id}'/'{p_name}' in void! Go rescue!");
            tell_user!(ctx.writer, "You are floating in the void…\n");
            return
        };

        let target_arc = {
            let r_lock = room.read().await;
            r_lock.exits.get(&dir).and_then(|e| e.upgrade())
        };

        let Some(target) = target_arc else {
            tell_user!(ctx.writer, "Alas, you have no means to go {}.\n", dir);
            return
        };

        if let Err(e) = Player::place_direct(plr.clone(), target.clone()).await {
            log::error!("Translocation failure: {e:?}");
            tell_user!(ctx.writer, "Strangely enough you cannot go there…\n");
        } else {
            LookCommand.exec({ctx.args = "";ctx}).await;
        }

        ctx.tx.send(Broadcast::Movement {
            from: room.clone().into(),
            to: target.clone().into(),
            who: plr.clone()
        }).ok();
    }
}
