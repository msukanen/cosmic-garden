//! A dummy for dummies.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, tell_user_unk};

pub struct DummyCommand;

#[async_trait]
impl Command for DummyCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        #[cfg(all(debug_assertions, feature = "localtest"))]{
            use crate::{tell_user, identity::IdentityQuery};

            let p = ctx.get_player_arc().unwrap();
            let actions_taken = {
                let mut p = p.write().await;
                p.actions_taken += 1;
                p.actions_taken
            };
            let loc = p.read().await.location.upgrade().unwrap();
            tell_user!(ctx.writer, "Actions: {}\n", actions_taken);
            tell_user!(ctx.writer, "  Where: {} ({})\n", loc.read().await.title(), loc.read().await.id());
            return;
        }
        tell_user_unk!(ctx.writer);
    }
}

#[cfg(test)]
mod cmd_dummy_tests {
    #[cfg(feature = "localtest")]
    #[tokio::test]
    async fn dummy_command() {
        use std::io::Cursor;

        use super::*;
        use crate::{cmd::{GotoCommand, look::LookCommand, redit::{ReditCommand, way::WayCommand}}, ctx, get_operational_mock_life, stabilize_threads, util::access::Access, world::world_tests::get_operational_mock_world};

        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),_) = get_operational_mock_world().await;
        get_operational_mock_life!(c,w);
        stabilize_threads!();

        let c = c.out;
        p.write().await.access = Access::Builder;
        state = ctx!(state, DummyCommand, "", s,c,w);
        state = ctx!(state, GotoCommand, "north", s,c,w);
        state = ctx!(state, ReditCommand, "this", s,c,w);
        state = ctx!(state, WayCommand, "north r-2", s,c,w);
        state = ctx!(state, GotoCommand, "north", s,c,w);
        stabilize_threads!(1);// to not send next command within next nanosecond…
        state = ctx!(state, DummyCommand, "", s,c,w);
        state = ctx!(state, LookCommand, "", s,c,w);
    }
}
