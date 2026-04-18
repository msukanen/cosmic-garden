//! List rooms…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, tell_user, validate_access};

pub struct ListCommand;

#[async_trait]
impl Command for ListCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let _ = validate_access!(ctx, builder);
        let mut report = String::from("<c green>--- The Existing Reality ---</c>\n");
        let rnum = {
            let w = ctx.world.read().await;
            for (id, rlock) in w.rooms.iter() {
                let r = rlock.read().await;
                report.push_str(&format!("  <c yellow>{}</c> : {}\n", id, r.title()));
            }
            w.rooms.len()
        };
        tell_user!(ctx.writer, "{}\nTotal: {} rooms found.\n", report, rnum);
    }
}

#[cfg(test)]
mod cmd_iedit_list {
    use crate::{cmd::{look::LookCommand, redit::{ReditCommand, list::ListCommand}}, ctx, io::ClientState, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn exits_listing() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut buffer);
        let (w, tx, ch, p) = get_operational_mock_world().await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, ReditCommand, "this", s, tx, ch, w, p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, ReditCommand, "this", s, tx, ch, w, p);
        let state = ctx!(state, LookCommand, "", s,tx,ch,w,p, |out:&str| out.contains("***"));
        let _ = ctx!(state, ListCommand, "", s,tx,ch,w,p, |out:&str| out.contains("r-1") && out.contains("r-2"));
    }
}
