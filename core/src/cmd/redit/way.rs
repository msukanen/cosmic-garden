//! Way - yay way - altering the ways of wayness.

use std::sync::Arc;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, roomloc_or_bust, show_help_if_needed, tell_user, util::direction::Direction, validate_access};

pub struct WayCommand;

// 'way <dir> <room-id>'
// 'way uni <dir> <room-id>' -- 'way u <dir> <room-id>'
// 'way rm <dir>'
// 'way bi rm <dir>'         -- '<way b rm | way br> <dir>'
#[async_trait]
impl Command for WayCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let r = roomloc_or_bust!(plr);
        show_help_if_needed!(ctx, "redit-way");

        let (dir, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        if args.is_empty() {
            show_help_if_needed!(ctx, "redit-way");
        }
        let dest = {
            let w = ctx.world.read().await;
            let Some(dest) = w.rooms.get(args) else {
                tell_user!(ctx.writer, "No such room exists. Check with <c yellow>list</c>…\n");
                return ;
            };
            dest.clone()
        };
        let dir = Direction::from(dir);
        let opp = dir.opposite();
        if let Err(_) = opp {
            tell_user!(ctx.writer, "Going to be a one-way trip by the looks of it…\nI can't deduct what's an opposite of '{}'!\n", dir);
        }
        {
            let mut rw = r.write().await;
            rw.exits.insert(dir.clone(), Arc::downgrade(&dest));
            tell_user!(ctx.writer, "'{}' linked to '{}'…\n", rw.id(), dest.read().await.id());
        }
        if let Ok(dir) = opp {
            let mut dw = dest.write().await;
            dw.exits.insert(dir.clone(), Arc::downgrade(&r));
            tell_user!(ctx.writer, "… and '{}' back-linked to '{}'…\n", dw.id(), r.read().await.id());
        }
    }
}

#[cfg(test)]
mod cmd_redit_way {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{goto::GotoCommand, look::LookCommand, pop::PopCommand, redit::way::WayCommand}, ctx, io::Broadcast, thread::life, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn way_creation_r1r2() {
        let mut buf: Vec<u8> = Vec::new();
        let mut s = Cursor::new(&mut buf);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let lt = tokio::spawn(life((c.out.clone(), c.recv.life), w.clone()));
        tokio::time::sleep(Duration::from_secs(2)).await; // let life() stabilize
        let state = ctx!(state, LookCommand, "", s,c.out,w,p);
        let state = ctx!(state, WayCommand, "east r-3",s,c.out,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, WayCommand, "east r-3",s,c.out,w,p,|out:&str| out.contains("No such room"));
        let state = ctx!(state, WayCommand, "balloon r-2",s,c.out,w,p,|out:&str| out.contains("one-way"));
        let state = ctx!(state, WayCommand, "north r-2",s,c.out,w,p,|out:&str| out.contains("back-link"));
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
        let _ = ctx!(state, LookCommand, "",s,c.out,w,p,|out:&str| out.contains("north") && out.contains("balloon"));
        c.out.shutdown().await;
        lt.await.ok();
    }
}
