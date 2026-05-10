//! Shutdown the server gracefully.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, show_help, show_help_if_needed, tell_user, validate_access};

pub struct ShutdownCommand;

#[async_trait]
impl Command for ShutdownCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let _ = validate_access!(ctx, admin);
        show_help_if_needed!(ctx, "shutdown");

        match ctx.args.to_lowercase().as_str() {
            "now" => {
                tell_user!(ctx.writer, "Broadcasting shutdown…\nBrace for impact…\n");
                ctx.state = crate::io::ClientState::Logout;
                ctx.out.janitor.send(crate::thread::SystemSignal::TimedShutdown { delay: 0 }).ok();
            }

            when => {
                let Ok(delay) = when.parse::<usize>() else {
                    tell_user!(ctx.writer, "<x warn>Close</x>… but no dice…\n");
                    show_help!(ctx, "u shutdown");
                };

                ctx.out.janitor.send(crate::thread::SystemSignal::TimedShutdown { delay }).ok();
            }
        }
    }
}



#[cfg(test)]
mod cmd_shutdown_tests {
    use std::{io::Cursor, time::Duration};
    use crate::{cmd::shutdown::ShutdownCommand, ctx, get_operational_mock_janitor, get_operational_mock_librarian, get_operational_mock_life, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn autoshutdown() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state, p),mut d) = get_operational_mock_world().await;
        let io_t = get_operational_mock_janitor!(c,w,d.0);
        let life_t = get_operational_mock_life!(c,w);
        let lib_t = get_operational_mock_librarian!(c,w);
        
        let mut autoshutdown = tokio::time::interval(Duration::from_secs(3));
        let mut autoshutdown_1st_tick = false;
        loop {
            tokio::select! {
                _ = autoshutdown.tick() => {
                    if autoshutdown_1st_tick {
                        p.write().await.access = Access::Admin;
                        state = ctx!(state, ShutdownCommand, "now", s,c.out,w);
                    } else {
                        log::debug!("Autoshutdown should happen in 3 seconds…");
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
