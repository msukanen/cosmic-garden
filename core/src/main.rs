//! Cosmic Garden — a multi-threaded MUD engine.
use std::sync::Arc;

use clap::Parser;

mod io;             use io::*;
mod io_thread;      use io_thread::io_thread;
mod life_thread;    use life_thread::life_thread;
use tokio::{io::{BufReader, AsyncBufReadExt}, net::TcpListener, sync::{RwLock, broadcast}};

use crate::{identity::IdentityQuery, io_thread::PLAYERS_TO_LOGOUT, string::{prompt::PromptType, sanitize::Sanitizer}, world::World};

mod cmd;
mod edit;
mod error;
mod identity;
mod item;
mod mob;
mod password;
mod player;
mod room;
mod string;
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
    // some const to deal with [World]-specific choices that aren't present for a reason or other…
    const GREETING: &'static str = "Welcome to Cosmic Garden!";
    const PROMPT_LOGIN: &'static str = "Login: ";
    const PROMPT_PWD1: &'static str = "Password: ";
    const PROMPT_PWDV: &'static str = "Re-type same pwd: ";
    const WELCOME_BACK: &'static str = "Welcome back!";
    const WELCOME_NEW: &'static str = "May your adventures be prosperous!";

    let _ = env_logger::try_init();
    let args = Cli::parse();
    let _ = DATA.set(args.data_path.clone());

    let world = World
        ::load_or_bootstrap(&args).await
        .unwrap_or_else(|err| {
            log::error!("{err:?}");
            panic!("World dead or in fire?! See logs…");
        });
    // connect some dots…
    world.link_rooms().await;
    let world = Arc::new(RwLock::new(world));

    tokio::spawn(life_thread());
    tokio::spawn(io_thread(world.clone(), args.clone()));

    // Create a listener that will accept incoming connections.
    let listen_on = format!("{}:{}", args.host_listen_addr, world.read().await.port);
    let listener = TcpListener::bind(&listen_on).await.unwrap();
    log::info!("Server listening on {}", listen_on);

    // A broadcast channel is used to send messages to all connected clients.
    // Here, we're just broadcasting chat messages.
    let (tx, _) = broadcast::channel::<Broadcast>(16);
    
    loop {
        // Wait for a new client to connect.
        let (socket, addr) = listener.accept().await.unwrap();
        log::info!("New connection from: {}", addr);

        // Sender broadcast clone…
        let tx = tx.clone();
        // World Arc…
        let world = world.clone();
        // Get a receiver for this client to listen for messages from others…
        let mut rx = tx.subscribe();

        // Spawn a new task to handle this client's connection,
        // which lets us to handle multiple clients concurrently.
        tokio::spawn(async move {
            // Split the socket into a reader and a writer.
            let (reader, mut writer) = socket.into_split();

            // Use a BufReader for efficient line-by-line reading.
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // Send a welcome message to the new client.
            let (greeting, login_prompt) = {
                let w = world.read().await;
                let g = w.greeting.clone().unwrap_or_else(|| GREETING.to_string());
                let p = w.fixed_prompts.get(&PromptType::Login).cloned().unwrap_or_else(|| PROMPT_LOGIN.to_string());
                (g, p)
            };
            tell_user!(&mut writer, "{}\n\n{}", greeting, &login_prompt);

            let mut state = ClientState::EnteringLogin;
            let mut abrupt_dc = false;

            //=======================================
            //
            // This is the main-loop for the client.
            //
            loop {
                // Check if [Player] is logging out (due disconnect or otherwise)…
                if let ClientState::Logout = &state {
                    let mut w = world.write().await;
                    if let Some(p) = w.players_by_sockaddr.remove(&addr) {
                        // drop the named mapping here as it's not needed for logout.
                        let lock = p.read().await;
                        let (id, name) = 
                            (lock.id().to_string(), lock.name.clone());
                        w.players_by_id.remove(lock.id());
                        if !abrupt_dc {
                            tell_user!(&mut writer, "\n<c cyan>Goodbye {}! See you soon again!</c>\n", lock.id());
                            log::trace!("Clean exit by '{id}'");
                        }
                        drop(lock);
                        let mut lock = (*PLAYERS_TO_LOGOUT).write().await;
                        lock.push(p);
                        log::trace!("Player '{name}' added to logout queue.");
                        drop(lock);
                    }
                    break;
                }

                // IMPORTANT: wipe the buffer before each read_line() as instead of
                //            clearing the buffer on its own, read_line() keeps
                //            accumulating onto it… we'd run out of memory sooner
                //            or later.
                line.clear();// ← !!!

                tokio::select! {
                    // --- First Branch: Read input from the client…
                    result = reader.read_line(&mut line) => {
                        // An abrupt disconnect?
                        if result.unwrap_or(0) == 0 {
                            log::info!("Client {} disconnected abruptly.", addr);
                            if state.is_in_game() {
                                abrupt_dc = true;
                                state = ClientState::Logout;
                                continue;
                            }
                            break; // not in game, cut the line, wipe the floors and take a break.
                        }

                        state = state.handle(&mut writer, world.clone(), &addr, &tx, &line.trim().sanitize()).await;
                    },

                    // --- Second Branch: Receive broadcast messages from other clients/system itself…
                    result = rx.recv() => (),
                }
            }
        });
    }
}
