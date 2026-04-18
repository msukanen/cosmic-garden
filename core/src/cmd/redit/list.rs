//! List rooms…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, tell_user, validate_access};

pub struct ListCommand;

#[async_trait]
impl Command for ListCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let _ = validate_access!(ctx, builder);
        let mut report = String::from("<c green>--- The Existing Reality ---</c>\n");
        let (what, page) = ctx.args.split_once(' ').unwrap_or((ctx.args, "1"));
        let Ok(mut page) = page.parse::<usize>() else {
            tell_user!(ctx.writer, "*cough* '{}' is not a valid number, by the by…\n", page);
            return ;
        };
        if page == 0 {
            page = 1;
        }
        let rooms = {
            let r = ctx.world.read().await;
            let rooms = r.paginated_room_entries(what, page, 10).await;
            for (id,t) in &rooms.entries {
                report.push_str(&format!("  <c yellow>{}</c> : <c cyan>{}</c>\n", id, t.read().await.title()));
            }
            rooms
        };
        tell_user!(ctx.writer, "{}\nTotal: {} matches ({page} of {} pages).\n", report, rooms.total_found, rooms.total_pages);
    }
}

#[cfg(test)]
mod cmd_iedit_list {
    use dicebag::InclusiveRandomRange;

    use crate::{cmd::{look::LookCommand, redit::{ReditCommand, list::ListCommand}}, ctx, io::ClientState, room::Room, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn exits_listing() {
        let mut b: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut b);
        let (w, tx, ch, p) = get_operational_mock_world().await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, ReditCommand, "this", s, tx, ch, w, p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, ReditCommand, "this", s, tx, ch, w, p);
        let state = ctx!(state, LookCommand, "", s,tx,ch,w,p, |out:&str| out.contains("***"));
        let _ = ctx!(state, ListCommand, "", s,tx,ch,w,p, |out:&str| out.contains("r-1") && out.contains("r-2"));
    }

    #[tokio::test]
    async fn exits_listing_parallel() {
        let mut b: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut b);
        let (w, tx, ch, p) = get_operational_mock_world().await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, ReditCommand, "this", s, tx, ch, w, p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, ReditCommand, "this", s, tx, ch, w, p);
        //let state = ctx!(state, LookCommand, "", s,tx,ch,w,p, |out:&str| out.contains("***"));
        let state = ctx!(state, ListCommand, "", s,tx,ch,w,p, |out:&str| out.contains("r-1") && out.contains("r-2") && !out.contains("r-3"));
        let mut lock = w.write().await;
        for i in 3..=1_000 {
            let id = format!("{}-{i}", ('a'..='z').random_of());
            let t = format!("Room #{i}");
            let d = format!("This would be the room #{i}");
            let r = Room::new(&id, &t).await.ok().unwrap();
            r.write().await.desc = d;
            lock.rooms.insert(id.clone(), r);
        }
        drop(lock);
        let state = ctx!(state, ListCommand, "r- 0", s,tx,ch,w,p, |out:&str| out.contains("Waterf"));
    }

}
