//! Cosmic Garden — a multi-threaded MUD engine.
extern crate cosmic_garden_pm;
use std::sync::Arc;

use clap::Parser;

mod io;             use convert_case::{Case, Casing};
use io::*;
use tokio::{net::TcpListener, sync::{RwLock, broadcast, mpsc}};

use crate::{cmd::cmd_alias::CMD_ALIASES, r#const::{DATA, WORLD}, thread::{SystemSignal, per_client::{self, PerClientData}, signal::SignalChannels}, world::World};

mod cmd;
pub mod combat;
mod r#const;
mod edit;
mod error;
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
pub(crate) struct Cli {
    #[arg(long, default_value = "0.0.0.0")] host_listen_addr: String,
    #[arg(long, default_value = "8080")] host_listen_port: u16,
    #[arg(long, default_value = "cosmic-garden")] world: String,
    #[arg(long, env = "COSMIC_GARDEN_DATA", default_value = "data")] data_path: String,
    #[arg(long)] bootstrap_url: Option<String>,
    #[arg(long)] autosave_queue_interval: Option<u64>,
}

/// The main culprit of many things main…
#[tokio::main]
async fn main() {
    let _ = env_logger::try_init();
    let args = Cli::parse();
    let _ = DATA.set(args.data_path.clone());
    let _ = WORLD.set(args.world.clone());

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
    let world = Arc::new(RwLock::new(world));

    // Establish system thread interconnection channels.
    let (janitor_tx, io_rx) = mpsc::channel::<SystemSignal>(32);
    let (librarian_tx, lib_rx) = mpsc::channel::<SystemSignal>(8);
    let (game_tx, game_rx) = mpsc::channel::<SystemSignal>(64);
    let private_channels = SignalChannels {
        janitor_tx: janitor_tx,
        librarian_tx: librarian_tx,
        game_tx: game_tx,
    };
    let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<()>();

    let io_t = tokio::spawn(thread::io::io_thread((private_channels.clone(), io_rx), world.clone(), args.clone().into(), done_tx));
    let life_t = tokio::spawn(thread::game::life_thread((private_channels.clone(), game_rx), world.clone()));
    let lib_t = tokio::spawn(thread::lib::librarian((private_channels.clone(), lib_rx)));

    // Create a listener that will accept incoming connections.
    let listen_on = format!("{}:{}", args.host_listen_addr, world.read().await.port);
    let listener = TcpListener::bind(&listen_on).await.unwrap();
    log::info!("{} v{} listening on {}", args.world.to_case(Case::Title), env!("CARGO_PKG_VERSION"), listen_on);

    // A broadcast channel is used to send e.g. messages to all connected clients.
    let (tx, _) = broadcast::channel::<Broadcast>(16);
    
    //
    // This is the main-loop for all …
    //
    loop {
        tokio::select! {
            conn = listener.accept() => {
                let (socket, addr) = conn.unwrap();
                log::info!("New connection from: {}", addr);
                let system_ch = private_channels.clone();
                let world = world.clone();

                // Get a between client I/O
                let (tx, rx) = (tx.clone(), tx.subscribe());

                let client_data = PerClientData {
                    socket, addr, system_ch, world, tx, rx
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

    io_t.await.ok();
    life_t.await.ok();
    lib_t.await.ok();
}
