//! Check or alter local weather…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, player_or_bust, room::environ::{WEATHER_CLEAR, WEATHER_RAIN}, roomloc_or_bust, tell_user, validate_access};

pub struct WeatherCommand;

#[async_trait]
impl Command for WeatherCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let loc = roomloc_or_bust!(plr);

        // show current weather
        if ctx.args.is_empty() {
            let env_bm = loc.read().await.special_env_bitmask();
            let state = if WEATHER_RAIN & env_bm != 0 {
                "rainy"
            } else if WEATHER_CLEAR & env_bm != 0 {
                "clear skies above"
            } else {
                "somewhat cloudy"
            };
            tell_user!(ctx.writer, "Currently it is {}.\n", state);
            return ;
        }

        let _ = validate_access!(ctx, true_builder);
        match &(ctx.args[..1]) {
            // rainy
            "r"|"R" => {
                let mut rw = loc.write().await;
                rw.set_special_env_bitmask(WEATHER_RAIN, false).ok();
                tell_user!(ctx.writer, "Weather now: <x info>rainy</x>.\n");
            }

            // clear; TODO: cloudy/clear distinction later
            "c"|"C" => {
                let mut rw = loc.write().await;
                rw.set_special_env_bitmask(WEATHER_CLEAR, false).ok();
                tell_user!(ctx.writer, "Weather now: <x info>clear</x>.\n");
            }

            // sunny
            // "s"|"S" => {
            //
            // }

            _ => err_tell_user!(ctx.writer, "Not a weatherman? '{}' is no weather pattern I'd be aware of…\n", ctx.args)
        }
    }
}

#[cfg(test)]
mod cmd_weather_tests {
    use std::io::Cursor;

    use crate::{cmd::weather::WeatherCommand, ctx, get_operational_mock_janitor, get_operational_mock_librarian, get_operational_mock_life, stabilize_threads, world::mock_world::get_operational_mock_world};

    #[tokio::test]
    async fn weather_currently() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),d) = get_operational_mock_world().await;
        get_operational_mock_janitor!(c,w,d.0);
        get_operational_mock_life!(c,w);
        get_operational_mock_librarian!(c,w);
        let c = c.out;
        stabilize_threads!();
        state = ctx!(state, WeatherCommand,"", s,c,w,|out:&str| out.contains("somewhat cloudy"));
        state = ctx!(state, WeatherCommand,"brainy",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = crate::util::access::Access::Player { event_host: false, builder: true };
        state = ctx!(state, WeatherCommand,"brainy",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = crate::util::access::Access::Builder;
        state = ctx!(state, WeatherCommand,"brainy",s,c,w,|out:&str| out.contains("no weather pattern"));
        _ = ctx!(state, WeatherCommand,"rainy",s,c,w,|out:&str| out.contains("Weather now") && out.contains("rainy"));

        #[cfg(feature = "stresstest")]{
        use sysinfo::System;
        use tokio::sync::broadcast::error::RecvError;
        use std::time::Duration;
        use crate::{io::Broadcast, thread::{SystemSignal, signal::SpawnType}};
        // Live RSS reporting:
        let mut rss_report_interval = tokio::time::interval(Duration::from_secs(2));
        let mut sys = System::new_all();
        let pid = sysinfo::get_current_pid().expect("Unable to determine PID?!");
        let mut peak_mem_kb: u64 = 0;
        let mut peak_counted = 0;
        let mut report_bcast = c.broadcast.subscribe();
        tokio::spawn(async move {
            loop {
            tokio::select! {
                _ = rss_report_interval.tick() => {
                    sys.refresh_memory();
                    if let Some(process) = sys.process(pid) {
                        peak_counted += 1;
                        let curr_mem_use = process.memory();
                        let kib = peak_mem_kb as f64 / 1024.0;
                        let mib = kib / 1024.0;
                        let gib = mib / 1024.0;
                        if curr_mem_use > peak_mem_kb {
                            peak_mem_kb = curr_mem_use;
                            log::info!("[TELEMETRY] peak mem usage: {gib:.2}GB ({mib:.2}MB; {kib:.2}KB)");
                            if gib > 40.0 {
                                log::warn!("[CRITICAL] Garden is occupying >40GB RAM.");
                            }
                        } else {
                            if peak_counted % 2 == 0 {
                                log::trace!("[MEM] usage: {gib:.2}GB ({mib:.2}MB; {kib:.2}KB)");
                            }
                        }
                    }
                },
            
                res = report_bcast.recv() => {
                    match res {
                        Ok(Broadcast::MessageInRoomE { message,.. }) => {
                            log::debug!("{message}");
                        }
                        Ok(_) => (),
                        Err(RecvError::Lagged(n)) => {
                            log::warn!("[TELEMETRY] Watcher lagged by {n} messages! The 100Hz firehose is too fast.");
                        }
                        Err(RecvError::Closed) => {
                            log::error!("[FAIL] Broadcast dead?")
                        }
                    }
                }}}
            }
        );
        let start_work = std::time::Instant::now();
        let (otx,orx) = tokio::sync::oneshot::channel::<bool>();
        c.life.send(SystemSignal::SpawnBatch { what: SpawnType::Mob { id: "goblin".into() }, num: 1_000_000, room: "r-1".into(), reply: otx.into() }).ok();
        // let the dust settle…
        let _ = orx.await;
        log::debug!("Dust?");
        let work_duration = start_work.elapsed();
        stabilize_threads!(60_000); // see for 30s what log fox says
        let spawns_per_sec = 1_000_000 as f64 / work_duration.as_secs_f64();
        let r1 = w.read().await.get_room_by_id("r-1").unwrap();
        let spawn_c = r1.read().await.entities().count();

        log::debug!("--terminated--");
        log::debug!("Duration: {work_duration:?} | Throughput: {spawns_per_sec:.2} ent/sec | Entities: {spawn_c}");
        
        }// stresstest
    }
}
