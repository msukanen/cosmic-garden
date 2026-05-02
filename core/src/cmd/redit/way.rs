//! Way - yay way - altering the ways of wayness.

use std::sync::{Arc, Weak};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, room::{Room, locking::Exit}, roomloc_or_bust, show_help, show_help_if_needed, tell_user, thread::SystemSignal, util::direction::{Direction, Directional}, validate_access};

pub struct WayCommand;

// 'way <dir> <room-id> [override]'
// 'way uni <dir> <room-id>' -- 'way u <dir> <room-id>'
// 'way rm <dir>'
// 'way bi rm <dir>'         -- '<way b rm | way br> <dir>'
#[async_trait]
impl Command for WayCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let origin = roomloc_or_bust!(plr);
        show_help_if_needed!(ctx, "way");

        let (dir, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        if args.is_empty() {
            tell_user!(ctx.writer, "Well, something's amiss…:\n");
            show_help!(ctx, "u way");
        }
        enum Op {
            Uni, BiRm, Rm, Default
        }
        let (op, dir, dest_id) = match dir {
            "u"|"uni" => {
                let (dir, dest_id) = args.split_once(' ').unwrap_or((args, ""));
                if dest_id.is_empty() {
                    tell_user!(ctx.writer, "Destination would be…? Where?\n");
                    show_help!(ctx, "u way");
                }
                (Op::Uni, dir, dest_id)
            }

            "rm" => (Op::Rm, dir, ""),
            "b"|"bi" => {
                let (rm, dir) = args.split_once(' ').unwrap_or((args, ""));
                if rm != "rm" {
                    tell_user!(ctx.writer, "What and a who now…?\n");
                    show_help!(ctx, "u way");
                }
                if dir.is_empty() {
                    tell_user!(ctx.writer, "Direction would be…? What exactly?\n");
                    show_help!(ctx, "u way");
                }
                (Op::BiRm, dir, "")
            }
            _ => (Op::Default, dir, args)
        };

        match op {
            Op::Uni => {
                make_uni_way(ctx, origin.clone(), dir, dest_id).await;
                tell_user!(ctx.writer, "Unidirectional exit '{}' grafted.\n", dir);
            },
            Op::Rm => {
                rm_uni_way(origin.clone(), dir).await;
                tell_user!(ctx.writer, "Exit '{}' removed.\n", dir);
            },
            Op::BiRm => rm_bi_way(ctx, origin.clone(), dir).await,
            Op::Default => make_bi_way(ctx, origin.clone(), dir, dest_id).await,
        };

        ctx.out.janitor.send(SystemSignal::SaveRoom { arc: origin }).ok();
    }
}

async fn rm_uni_way(origin: Arc<RwLock<Room>>, dir: &str) {
    let dir = Direction::from(dir);
    origin.write().await.remove_exit(&dir);
}

async fn rm_bi_way(ctx: &mut CommandCtx<'_>, origin: Arc<RwLock<Room>>, dir: &str) {
    let d = Direction::from(dir);
    let opp = d.opposite();
    // grab the neighbor before dir is eradicated
    let neighbor = origin.read().await.exits.get(&d).and_then(|e| e.as_arc());
    rm_uni_way(origin, dir).await;
    if let (Some(n_arc), Ok(opp)) = (neighbor, opp) {
        rm_uni_way(n_arc, &opp.to_string()).await;
        tell_user!(ctx.writer, "Bidirectional link {} ↔ {} severed.\n", dir, opp);
    } else {
        tell_user!(ctx.writer, "Exit {} removed. No recognizeable bidirectional to break was found…\n", dir);
    }
}

async fn make_uni_way(ctx: &mut CommandCtx<'_>, origin: Arc<RwLock<Room>>, dir: &str, dest_id: &str) {
    let dir = Direction::from(dir);
    let exit = Exit::Free { room:
        if let Some(d_arc) = ctx.world.read().await.get_room_by_id(dest_id) {
            Arc::downgrade(&d_arc)
        } else { Weak::new() }
    };
    origin.write().await.assign_exit(dir, exit).await;
}

async fn make_bi_way(ctx: &mut CommandCtx<'_>, origin: Arc<RwLock<Room>>, dir: &str, dest_id: &str) {
    let (dest_id, ovr_bdir) = dest_id.split_once(' ').unwrap_or((dest_id, ""));
    make_uni_way(ctx, origin.clone(), dir, dest_id).await;
    if dest_id.is_empty() {
        tell_user!(ctx.writer, "Mirage exit '{}' evoked…\n", dir);
        return;
    }
    if let Ok(opp) = dir.opposite() {
        if let Some(d_arc) = ctx.world.read().await.get_room_by_id(dest_id) {
            {
                let mut dw = d_arc.write().await;
                if ovr_bdir != "override" && dw.contains_exit(&opp) {
                    err_tell_user!(ctx.writer, "Shucks! Exit '{}' created, but destination '{}' already has an assigned opposite.\nUse <c yellow>way <dir> <room-id> override</c> to force redirection.\n", dir, dest_id);
                }
                let exit = Exit::Free { room: Arc::downgrade(&origin) };
                dw.assign_exit(opp.clone(), exit).await;
            }
            tell_user!(ctx.writer, "Bidirectional link {} ↔ {} established.\n", dir, opp);
            ctx.out.janitor.send(SystemSignal::SaveRoom { arc: d_arc }).ok();
        } else {
            err_tell_user!(ctx.writer, "Ack! Destination '{}' doesn't *actually* exist!\nExit '{}' nonetheless produced for later use…\n", dest_id, dir);
        }
    } else {
        err_tell_user!(ctx.writer, "Can't deduct opposite for non-cardinal '{}'!\nNeed manual <c yellow>way uni <dir> <room-id></c> issued at '{}'.\n", dir, dest_id);
    }
}

#[cfg(test)]
mod cmd_redit_way {
    use std::io::Cursor;

    use crate::{cmd::{goto::GotoCommand, look::LookCommand, pop::PopCommand, redit::way::WayCommand}, ctx, get_operational_mock_life, io::Broadcast, stabilize_threads, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn way_creation_r1r2() {
        let mut buf: Vec<u8> = Vec::new();
        let mut s = Cursor::new(&mut buf);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let lt = get_operational_mock_life!(c,w);
        stabilize_threads!(100);
        let state = ctx!(state, LookCommand, "", s,c.out,w,p);
        let state = ctx!(sup true, state, WayCommand, "east r-3",s,c.out,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, WayCommand, "east r-3",s,c.out,w,p,|out:&str| out.contains("actually"));
        let state = ctx!(state, WayCommand, "balloon r-2",s,c.out,w,p,|out:&str| out.contains("deduct"));
        let state = ctx!(state, WayCommand, "north r-2",s,c.out,w,p,|out:&str| out.contains("Bidirectional link"));
        let state = ctx!(state, LookCommand, "",s,c.out,w,p,|out:&str| out.contains("north") && out.contains("balloon"));
        let mut rx = c.out.broadcast.subscribe();
        let state = ctx!(state, GotoCommand, "north",s,c.out,w,p);
        let state = if let Ok(Broadcast::Force { command, .. }) = rx.recv().await {
            if command == "look" {
                ctx!(state, LookCommand, "", s,c.out,w,p)
            } else { state }} else { state };
        let state = ctx!(state, GotoCommand, "south",s,c.out,w,p);
        let state = if let Ok(Broadcast::Force { command, .. }) = rx.recv().await {
            if command == "look" {
                ctx!(state, LookCommand, "", s,c.out,w,p)
            } else { state }} else { state };
        let state = ctx!(state, GotoCommand, "balloon",s,c.out,w,p);
        let state = if let Ok(Broadcast::Force { command, .. }) = rx.recv().await {
            if command == "look" {
                ctx!(state, LookCommand, "", s,c.out,w,p,|out:&str| out.contains("south"))
            } else { state }} else { state };
        let state = ctx!(state, PopCommand, "balloon",s,c.out,w,p,|out:&str| out.contains("falling"));
        let _ = ctx!(state, LookCommand, "",s,c.out,w,p,|out:&str| out.contains("north") && !out.contains("balloon"));
        c.out.shutdown().await;
        lt.await.ok();
    }
}
