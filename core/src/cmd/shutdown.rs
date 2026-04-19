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
        ctx.tx.send(crate::io::Broadcast::Shutdown).ok();
        ctx.system.shutdown().await;
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
        let (w,tx,ch,p) = get_operational_mock_world().await;
        let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<()>();
        let io_t = tokio::spawn(crate::thread::io::io_thread((ch.0.clone(), ch.1.janitor_rx), w.clone(), None, done_tx));
        let life_t = tokio::spawn(crate::thread::game::life_thread((ch.0.clone(), ch.1.game_rx), w.clone()));
        let lib_t = tokio::spawn(crate::thread::lib::librarian((ch.0.clone(), ch.1.librarian_rx)));
        
        let mut autoshutdown = tokio::time::interval(Duration::from_secs(10));
        let mut autoshutdown_1st_tick = false;
        loop {
            tokio::select! {
                _ = autoshutdown.tick() => {
                    if autoshutdown_1st_tick {
                        let state = ClientState::Playing { player: p.clone() };
                        p.write().await.access = Access::Admin;
                        let _ = ctx!(state, ShutdownCommand, "", s,tx,ch,w,p);
                    } else {
                        log::debug!("Autoshutdown should happen in 10 seconds…");
                        autoshutdown_1st_tick = true;
                    }
                }

                _ = &mut done_rx => {
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
