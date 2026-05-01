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
            for (m_id, arc) in &rooms.entries {
                let lock = arc.read().await;
                report.push_str(&format!("  <c yellow>{}</c> <c gray>:</c> {} <c gray>:</c> <c cyan>{}</c>\n", m_id, lock.id(), lock.title()));
            }
            rooms
        };
        tell_user!(ctx.writer, "{}\nTotal: {} matches ({page} of {} pages).\n", report, rooms.total_found, rooms.total_pages);
    }
}

#[cfg(test)]
mod cmd_redit_list {
    use dicebag::InclusiveRandomRange;

    use crate::{cmd::{look::LookCommand, redit::{ReditCommand, list::ListCommand}}, ctx, identity::MachineIdentity, room::Room, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn exits_listing() {
        let mut b: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let state = ctx!(state, ReditCommand, "this", s, c.out, w, p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, ReditCommand, "this", s, c.out, w, p);
        let state = ctx!(state, LookCommand, "", s,c.out,w,p, |out:&str| out.contains("***"));
        let _ = ctx!(state, ListCommand, "", s,c.out,w,p, |out:&str| out.contains("r-1") && out.contains("r-2"));
    }

    #[tokio::test]
    async fn exits_listing_parallel() {
        let mut b: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let state = ctx!(state, ReditCommand, "this", s, c.out, w, p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, ReditCommand, "this", s, c.out, w, p);
        let state = ctx!(state, ListCommand, "", s,c.out,w,p, |out:&str| out.contains("Waterfall") && out.contains("Incineration"));
        let mut lock = w.write().await;
        for i in 3..=1_000 {
            let id = format!("{}-{i}", ('a'..='z').random_of());
            let t = format!("Room #{i}");
            let d = format!("This would be the room #{i}");
            let r = Room::new(&id, &t).await.ok().unwrap();
            r.write().await.desc = d;
            lock.insert_room(r);
        }
        drop(lock);
        let _ = ctx!(state, ListCommand, "r-1 0", s,c.out,w,p, |out:&str| out.contains("Inciner"));
    }

}
