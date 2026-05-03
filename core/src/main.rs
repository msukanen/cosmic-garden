//! Cosmic Garden — a multi-threaded MUD engine.
extern crate cosmic_garden_pm;
use std::{sync::Arc, time::Duration};

use clap::Parser;

mod io;             use convert_case::{Case, Casing};
use sysinfo::System;
use tokio::{net::TcpListener, sync::RwLock};

use crate::{cmd::cmd_alias::CMD_ALIASES, r#const::{DATA, WORLD}, thread::{life::{BATTLE_HZ, CORE_HZ}, per_client::{self, PerClientData}, signal::SignalChannels}, world::World};

mod cmd;
pub mod combat;
mod r#const;
mod edit;
mod error;
mod help;
mod identity;
mod item;
#[macro_use]
  mod macros;
mod mob;
mod password;
mod player;
mod room;
mod serial;
mod string;
mod thread;
mod traits;
mod user;
mod util;
mod world;

/// Command line options…
#[derive(Debug, Parser, Clone)]
#[command(
    version,
    about = "Cosmic Garden MUD Engine.",
//    after_help = ""
)]
pub struct Cli {
    #[arg(long, default_value = "0.0.0.0")] host_listen_addr: String,
    #[arg(long, default_value = "8080")] host_listen_port: u16,
    #[arg(long, default_value = "cosmic-garden")] world: String,
    #[arg(long, env = "COSMIC_GARDEN_DATA", default_value = "data")] data_path: String,
    #[arg(long)] bootstrap_url: Option<String>,
    #[arg(long)] autosave_queue_interval: Option<u64>,
    #[arg(long, default_value = "100")] core_hz: u8,
    #[arg(long, default_value = "50")] battle_hz: u8,
}

/// The main culprit of many things main…
#[tokio::main]
async fn main() {
    let _ = env_logger::try_init();
    let args = Cli::parse();
    let _ = DATA.set(args.data_path.clone());
    let _ = WORLD.set(args.world.clone());
    let _ = CORE_HZ.set(args.core_hz.clamp(5, 100));//    5-100 Hz
    let _ = BATTLE_HZ.set(args.battle_hz.clamp(5, 75));// 5-75 Hz

    if (*CMD_ALIASES).is_empty() {
        log::info!("No command aliases defined yet.");
    } else {
        log::info!("Command aliases instantiated.");
    };

    let mut world = World
        ::load_or_bootstrap(&args).await
        .unwrap_or_else(|err| {
            log::error!("{err:?}");
            panic!("World dead or in fire?! See logs…");
        });
    // connect some dots…
    world.link_rooms().await;

    // Establish system thread interconnection channels.
    let priv_chs = SignalChannels::default();
    let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<()>();
    world.channels = Some(priv_chs.out.clone());

    // Shared world, shared fun!
    let world = Arc::new(RwLock::new(world));

    let jan_t = tokio::spawn(thread::janitor((priv_chs.out.clone(), priv_chs.recv.janitor), world.clone(), args.clone().into(), done_tx));
    let life_t = tokio::spawn(thread::life((priv_chs.out.clone(), priv_chs.recv.life), world.clone(), (args.core_hz, args.battle_hz)));
    let lib_t = tokio::spawn(thread::librarian((priv_chs.out.clone(), priv_chs.recv.librarian), world.clone()));

    // Create a listener that will accept incoming connections.
    let listen_on = format!("{}:{}", args.host_listen_addr, world.read().await.port);
    let listener = TcpListener::bind(&listen_on).await.unwrap();
    log::info!("{} v{} listening on {}", args.world.to_case(Case::Title), env!("CARGO_PKG_VERSION"), listen_on);

    // Live RSS reporting:
    let mut rss_report_interval = tokio::time::interval(Duration::from_secs(5));
    let mut sys = System::new_all();
    let pid = sysinfo::get_current_pid().expect("Unable to determine PID?!");
    let mut peak_mem_kb: u64 = 0;
    let mut peak_counted = 0;
    //
    // This is the main-loop for all …
    //
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
                        if peak_counted % 6 == 0 {
                            log::trace!("[MEM] usage: {gib:.2}GB ({mib:.2}MB; {kib:.2}KB)");
                        }
                    }
                }
            }

            conn = listener.accept() => {
                let (socket, addr) = conn.unwrap();
                log::info!("New connection from: {}", addr);
                let out = priv_chs.out.clone();
                let world = world.clone();
                let client_data = PerClientData {
                    socket, addr, world,
                    rx: out.broadcast.subscribe(),
                    out,
                };

                // Spawn a new task to handle this client's connection,
                // = handle multiple clients concurrently.
                tokio::spawn(async move { per_client::per_client_thread( client_data ).await });
            }

            _ = &mut done_rx => {
                break;
            }
        }
    }

    jan_t.await.ok();
    life_t.await.ok();
    lib_t.await.ok();
}
