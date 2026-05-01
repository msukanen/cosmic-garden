//! Diggy, diggy.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, redit::ReditCommand}, identity::{MachineIdentity, uniq::UuidValidator}, room::Room, tell_user, thread::SystemSignal, translocate, util::direction::Direction, validate_access};

pub struct DigCommand;

#[async_trait]
impl Command for DigCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        if ctx.args.is_empty() {
            tell_user!(ctx.writer, "Dig where exactly?\n");
            return;
        }
        let Some(args) = ctx.args.split_once(' ') else {
            tell_user!(ctx.writer, "Destination ID?\nUsage: <c yellow>dig <dir> <dest-id></c>\n");
            return;
        };
        // dir will be either a cardinal direction or Custom(..)
        let dir = Direction::from(args.0);
        let Ok(dest_id) = args.1.as_id() else {
            tell_user!(ctx.writer, "That doesn't work as an ID. Try again…\n");
            return ;
        };
        // does the world already have `dest_id`?
        let dest_room = {
            let w = ctx.world.read().await;
            w.rooms.get(&dest_id.as_m_id()).cloned()
        };

        let target_arc = if let Some(existing) = dest_room {
            // world knows the destination, just make a new bridge.
            existing
        } else {
            let new_title = format!("New Room ({})", dest_id);
            match Room::new(&dest_id, &new_title).await {
                Ok(r) => {
                    let mut w = ctx.world.write().await;
                    w.rooms.insert(dest_id.as_m_id(), r.clone());
                    let _ = r.read().await.save();
                    r
                },
                Err(e) => {
                    log::warn!("Digging accident: {e:?}");
                    tell_user!(ctx.writer, "Nope, ground is too hard here. Cannot dig!\n");
                    return ;
                }
            }
        };

        let origin_arc = {
            let p = plr.read().await;
            p.location.upgrade().expect("Builder floating in the void. Eject!")
        };

        if let Err(e) = origin_arc.write().await.link_exit(ctx.world.clone(), dir.clone(), &dest_id).await {
            tell_user!(ctx.writer, "Symmetry-solder failed: {:?}.\nEither leave it at that (unidirectional, if you so intended) or create return point manually.\n", e);
        } else {
            // persist both rooms
            let _ = origin_arc.read().await.save().await;
            let _ = target_arc.read().await.save().await;
            // ping the janitor. Even if they don't respond right now, they'll save the world soon enough anyway.
            ctx.out.janitor.send(SystemSignal::SaveWorld).ok();
            tell_user!(ctx.writer, "Diggy diggy to {} — success!\n", dir);
            translocate!(plr, origin_arc, target_arc);
            ReditCommand.exec({ctx.args = "this";ctx}).await;
        }
    }
}
