//! Shutdown the server gracefully.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, tell_user, validate_access};

pub struct ShutdownCommand;

#[async_trait]
impl Command for ShutdownCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let _ = validate_access!(ctx, admin);
        tell_user!(ctx.writer, "Broadcasting shutdown…\nBrace for impact…\n");
        ctx.state = crate::io::ClientState::Logout;
        ctx.out.broadcast.send(crate::io::Broadcast::Shutdown).ok();
        ctx.out.shutdown().await;
    }
}

#[cfg(test)]
mod cmd_shutdown_tests {
    use std::{io::Cursor, time::Duration};
    use crate::{cmd::shutdown::ShutdownCommand, ctx, io::ClientState, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn autoshutdown() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(_, p),mut d) = get_operational_mock_world().await;
        let io_t = tokio::spawn(crate::thread::janitor((c.out.clone(), c.recv.janitor), w.clone(), None, d.0));
        let life_t = tokio::spawn(crate::thread::life((c.out.clone(), c.recv.life), w.clone()));
        let lib_t = tokio::spawn(crate::thread::librarian((c.out.clone(), c.recv.librarian), w.clone()));
        
        let mut autoshutdown = tokio::time::interval(Duration::from_secs(10));
        let mut autoshutdown_1st_tick = false;
        loop {
            tokio::select! {
                _ = autoshutdown.tick() => {
                    if autoshutdown_1st_tick {
                        let state = ClientState::Playing { player: p.clone() };
                        p.write().await.access = Access::Admin;
                        let _ = ctx!(state, ShutdownCommand, "", s,c.out,w,p);
                    } else {
                        log::debug!("Autoshutdown should happen in 10 seconds…");
                        autoshutdown_1st_tick = true;
                    }
                }

                _ = &mut d.1 => {
                    break;
                }
            }
        }
        io_t.await.ok();
        life_t.await.ok();
        lib_t.await.ok();
        log::info!("--terminated--");
    }
}
