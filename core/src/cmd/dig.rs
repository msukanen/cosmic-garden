//! Diggy, diggy.

use std::sync::Arc;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, redit::ReditCommand}, combat::Battler, err_tell_user, identity::{MachineIdentity, uniq::UuidValidator}, room::{Room, locking::Exit}, roomloc_or_bust, show_help, show_help_if_needed, tell_user, thread::SystemSignal, translocate, util::direction::Direction, validate_access};

pub struct DigCommand;

#[async_trait]
impl Command for DigCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        show_help_if_needed!(ctx, "dig");
        let origin_arc = roomloc_or_bust!(plr);

        let (dir, dest) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        if dest.is_empty() {
            tell_user!(ctx.writer, "Destination ID?\n");
            show_help!(ctx, "u dig");
        };
        let Ok(dest_id) = dest.as_id() else {
            tell_user!(ctx.writer, "'{}' doesn't work as an ID. Try again…\n", dest);
            return ;
        };

        // dir will be either a cardinal direction or Custom(..)
        let dir = Direction::from(dir);

        // does the world already know `dest_id`?
        let mut pre_existing = false;
        let d_arc = if let Some(d_arc) = ctx.world.read().await.get_room_by_id(&dest_id).clone() {
            pre_existing = true;
            d_arc
        } else {
            // make a new room, no `dest_id` present yet.
            let new_title = format!("New Room ({})", dest_id);
            match Room::new(&dest_id, &new_title).await {
                Ok(d_arc) => {
                    // try tell the World about us!
                    let mut w = ctx.world.write().await;
                    if let Err(_) = w.insert_room_by_m_id(dest_id.as_m_id(), dest_id.clone(), d_arc.clone()) {
                        err_tell_user!(ctx.writer, "Unfortunately there was ID clash… Fix the ID?\n");
                    }
                    
                    let exit = Exit::Free { room: Arc::downgrade(&d_arc) };
                    origin_arc.write().await.assign_exit(dir.clone(), exit).await;
                    d_arc
                },
                Err(e) => {
                    log::warn!("Digging accident: {e:?}");
                    err_tell_user!(ctx.writer, "Nope, ground is too hard here. Cannot dig!\n");
                }
            }
        };

        let opp = dir.opposite();
        // symmetric dig…?
        if pre_existing {
            if opp.is_err() {
                err_tell_user!(ctx.writer, "Exit to <c cyan>{}</c> grafted, but it's <c red>unidirectional</c>.\nIf wanted/needed, go <c yellow>dig</c> or <c yellow>way</c> a return route manually.\n", dest_id);
            }
            let opp = opp.unwrap();
            // lets not overwrite already existing exit…
            {
                let mut d = d_arc.write().await;
                if d.contains_exit(&opp) {
                    err_tell_user!(ctx.writer, "Exit '<c cyan>{}</c>' grafted, but <c cyan>{}</c>'s corresponding <c cyan>{}</c> is already occupied.\nYou need to sort out return direction manually there, if needed.\n", dir, dest_id, opp);
                }
                let exit = Exit::Free { room: Arc::downgrade(&origin_arc) };
                d.assign_exit(opp.clone(), exit).await;
            }
            ctx.out.janitor.send(SystemSignal::SaveRoom { arc: d_arc }).ok();
            tell_user!(ctx.writer, "Bidirectional exit {} ↔ {} grafted.\n", dir, opp);
            return;
        }

        if let Ok(opp) = opp {
            let exit = Exit::Free { room: Arc::downgrade(&origin_arc) };
            tell_user!(ctx.writer, "Bidirectional exit {} ↔ {} grafted.\n\n", dir, opp);
            d_arc.write().await.assign_exit(opp, exit).await;
        } else {
            tell_user!(ctx.writer, "Custom <c red>unidirectional</c> exit <c cyan>{}</c> has no direct opposite.\nYou need to make one manually at <c cyan>{}</c> …\n", dir, dest_id);
        }

        // bypass life-thread judgement and just in case wires got tangled, abort combat…:
        ctx.out.life.send(SystemSignal::AbortBattleNow { who: plr.clone() as Battler }).ok();
        translocate!(plr, origin_arc, d_arc);
        plr.write().await.last_goto = None;
        ReditCommand.exec({ctx.args = "--dig";ctx}).await;
    }
}

#[cfg(test)]
mod cmd_dig_tests {
    use std::io::Cursor;
    use super::*;
    use crate::{ctx, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn dig_third_cardinal() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),_) = get_operational_mock_world().await;
        let c = c.out;
        // note: dig bypasses life-thread judgement.
        state = ctx!(sup state, DigCommand, "",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        state = ctx!(state, DigCommand, "east r-3",s,c,w);
        state = ctx!(state, DigCommand, "east r-3",s,c,w);
    }

    #[tokio::test]
    async fn dig_custom() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),_) = get_operational_mock_world().await;
        let c = c.out;
        // note: dig bypasses life-thread judgement.
        state = ctx!(sup state, DigCommand, "teleport r-4",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        state = ctx!(state, DigCommand, "teleport r-4",s,c,w);
        state = ctx!(state, DigCommand, "teleport r-4",s,c,w);
    }
}
