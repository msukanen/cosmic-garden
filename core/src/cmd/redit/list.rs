//! List rooms…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, string::styling::MAX_DESCRIPTION_LINES, tell_user, validate_access, world::room_list};

pub struct ListCommand;

#[async_trait]
impl Command for ListCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        const PAGE_COLS: usize = 1;
        const ENT_PER_PAGE: usize = MAX_DESCRIPTION_LINES * PAGE_COLS;

        let _ = validate_access!(ctx, builder);
        let (what, page) = ctx.args.split_once(' ').unwrap_or((ctx.args, "1"));
        let mut page_num: usize;
        let term = if let Ok(num_what) = what.parse::<usize>() {
            page_num = num_what;
            None
        } else {
            let Ok(page) = page.parse::<usize>() else {
                tell_user!(ctx.writer, "*cough* '{}' is not a valid number, by the by…\n", page);
                return;
            };
            page_num = page;
            Some(what.to_string())
        };

        let mut report = String::from("<c green>--- The Existing Reality ---</c>\n");
        let (total_found, total_pages) = {
            let mut rooms = room_list(&ctx.world, term).await;
            let r_len = rooms.len();
            let total_pages = (r_len + (ENT_PER_PAGE - 1)) / ENT_PER_PAGE;
            page_num = page_num.min(total_pages).saturating_sub(1);
            let mut entry = 0;
            let total_found = rooms.len();
            if page_num > 0 {
                rooms.drain(0..(ENT_PER_PAGE * page_num) - 1);
            };
            for (m_id, arc) in &rooms {
                let lock = arc.read().await;
                report.push_str(&format!("  <c yellow>{:>20}</c> <c gray>:</c> {} <c gray>:</c> <c cyan>{}</c>\n", m_id, lock.id(), lock.title()));
                entry += 1;
                if entry >= ENT_PER_PAGE { break; }
            }
            (total_found, total_pages)
        };
        tell_user!(ctx.writer, "{report}\nTotal: {} matches ({} of {} pages).\n", total_found, page_num+1, total_pages);
    }
}

#[cfg(test)]
mod cmd_redit_list {
    use crate::{cmd::{redit::{ReditCommand, list::ListCommand}}, ctx, room::Room, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn exits_listing() {
        let mut b: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut b);
        let (w,c,(mut state, p),_) = get_operational_mock_world().await;
        let c = c.out;
        state = ctx!(state, ReditCommand, "this", s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        state = ctx!(state, ReditCommand, "this", s,c,w);
        state = ctx!(state, ListCommand, "", s,c,w,|out:&str| out.contains("Waterfall") && out.contains("Incineration"));
        let mut lock = w.write().await;
        
        let alphabet: Vec<char> = ('a'..='z').collect();
        let alphalen = alphabet.len();
        for i in 1..=1_000 {
            let id = format!("{}-{i}", alphabet[i % alphalen]);
            let t = format!("Room #{i}");
            let d = format!("This would be the room #{i}");
            let r = Room::new(&id, &t).await.ok().unwrap();
            r.write().await.desc = d;
            lock.insert_room(r).await.ok();
        }
        drop(lock);
        state = ctx!(state, ListCommand, "40", s,c,w,|out:&str| out.contains("matches (40"));
        state = ctx!(state, ListCommand, "^o-40$", s,c,w,|out:&str| out.contains("Room #40") && out.contains("Total: 1 m") && !out.contains("#404"));
        state = ctx!(state, ListCommand, "^o-40", s,c,w,|out:&str| out.contains("Room #40") && out.contains("Total: 2 m") && out.contains("#404"));
        state = ctx!(state, ListCommand, "21$", s,c,w,|out:&str| out.contains("Room #21") && out.contains("Total: 10 m")
            && out.contains("#121")
            && out.contains("#221")
            && out.contains("#321")
            && out.contains("#421")
            && out.contains("#521")
            && out.contains("#621")
            && out.contains("#721")
            && out.contains("#821")
            && out.contains("#921")
        );
    }
}
