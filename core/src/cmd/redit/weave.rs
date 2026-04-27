//! Weave the edits into fabric of the running World.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, redit::abort::AbortCommand}, io::Broadcast, thread::janitor::ROOMS_TO_SAVE, validate_editor_mode, player_or_bust, tell_user};

pub struct WeaveCommand;

#[async_trait]
impl Command for WeaveCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        validate_editor_mode!(ctx, "REdit");
        if !ctx.state.is_dirty() {
            tell_user!(ctx.writer, "You weave your hands, but nothing happens…\nProbably because there was no modifications pending.\n");
            return ;
        }

        let (room_arc, wip_copy) = {
            let p = plr.read().await;
            let room = p.location.upgrade().unwrap();
            let wip = p.redit_buffer.as_ref().unwrap().clone();
            (room, wip)
        };

        {
            let mut lock = room_arc.write().await;
            lock.copyback(wip_copy);
            drop(lock);
            (*ROOMS_TO_SAVE).write().await.push(room_arc.clone());
        }

        tell_user!(ctx.writer, "<c green>Reality is being rewritten…\n");
        let rooms: Vec<_> = vec![room_arc.clone()];
        ctx.out.broadcast.send(Broadcast::System {
            from: None,
            rooms,
            message: "<c yellow>The reality shifts around you!</c>".into(),
        }).ok();
        
        AbortCommand.exec({ctx.args = "quiet"; ctx}).await;
    }
}
