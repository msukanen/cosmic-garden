//! Cosmic Garden — a multi-threaded MUD engine.
use std::sync::Arc;

use clap::Parser;

mod io;             use convert_case::{Case, Casing};
use io::*;
use tokio::{io::{AsyncBufReadExt, BufReader}, net::TcpListener, sync::{RwLock, broadcast}};

use crate::{cmd::{CommandCtx, cmd_alias::CMD_ALIASES}, identity::IdentityQuery, string::{prompt::PromptType, sanitize::Sanitizer}, thread::janitor::PLAYERS_TO_LOGOUT, world::World};

mod cmd;
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
    // some const to deal with [World]-specific choices that aren't present for a reason or other…
    const GREETING: &'static str = "Welcome to Cosmic Garden!";
    const PROMPT_LOGIN: &'static str = "Login: ";

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

    tokio::spawn(thread::io::io_thread(world.clone(), args.clone()));
    tokio::spawn(thread::game::life_thread(world.clone()));
    tokio::spawn(thread::lib::librarian());

    // Create a listener that will accept incoming connections.
    let listen_on = format!("{}:{}", args.host_listen_addr, world.read().await.port);
    let listener = TcpListener::bind(&listen_on).await.unwrap();
    log::info!("{} v{} listening on {}", args.world.to_case(Case::Title), env!("CARGO_PKG_VERSION"), listen_on);

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
                            tell_user!(&mut writer, "\n<c cyan>Goodbye {}! See you soon again!</c>\n", lock.title());
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
                    result = rx.recv() => match state.clone() {
                        ClientState::Playing { player } |
                        ClientState::Editing { player, .. } => match result {
                            Ok(bcast) => match bcast {
                                Broadcast::Say { room, message, from } => {
                                    if !Arc::ptr_eq(&from, &player) {
                                        let Some(ploc) = player.read().await.location.upgrade() else {continue;};
                                        if Arc::ptr_eq(&room, &ploc) {
                                            let title = from.read().await.title().to_string();
                                            tell_user!(&mut writer, "\n<c blue>[<c cyan>{}</c>]</c> says: \"{}\"\n", title, message);
                                            reprompt_playing_user!(writer, state);
                                        }
                                    }
                                },

                                Broadcast::Movement { to, from, who } => {
                                    // no need to tell yourself that you just switched rooms…
                                    if Arc::ptr_eq(&who, &player) { continue; }
                                    // in the void...?
                                    let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                                    if Arc::ptr_eq(&to, &ploc) {
                                        let who = who.read().await.title().to_string();
                                        tell_user!(&mut writer, "\n<c cyan>{}</c> arrives…\n", who);
                                    } else if Arc::ptr_eq(&from, &ploc) {
                                        let who = who.read().await.title().to_string();
                                        tell_user!(&mut writer, "\n<c cyan>{}</c> departs…\n", who);
                                    } else {
                                        // were weren't at either end-point...
                                        continue;
                                    }
                                    reprompt_playing_user!(writer, state);
                                },

                                Broadcast::Logout { from, who } => {
                                    let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                                    if Arc::ptr_eq(&from, &ploc) {
                                        tell_user!(&mut writer, "\n<c cyan>{}</c> vanishes into the mists…\n", who);
                                        reprompt_playing_user!(writer, state);
                                    }
                                },

                                Broadcast::System { rooms, message, sender } => {
                                    let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                                    if let Some(sender) = sender {
                                        // we'll ignore system messages we sent ourselves
                                        if Arc::ptr_eq(&player, &sender) { continue; }
                                    }
                                    for room in rooms {
                                        if Arc::ptr_eq(&room, &ploc) {
                                            tell_user!(&mut writer, "\n{}\n", message);
                                            reprompt_playing_user!(writer, state);
                                            break;
                                        }
                                    }
                                },

                                Broadcast::BiSignal { to, from, who, message_to, message_from, message_who } => {
                                    // am I the 'who'?
                                    if Arc::ptr_eq(&player, &who) {
                                        tell_user!(&mut writer, "\n{}\n", message_who);
                                        reprompt_playing_user!(writer, state);
                                        continue;
                                    }
                                    // just skip if in void
                                    let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                                    if !Arc::ptr_eq(&to, &ploc) && !Arc::ptr_eq(&from, &ploc) { continue; }
                                    tell_user!(&mut writer, "\n{}\n", if Arc::ptr_eq(&to, &ploc) { message_to } else { message_from} );
                                    reprompt_playing_user!(writer, state);
                                },

                                Broadcast::Force { command, who, by, delivery } => {
                                    static UNK_FORCE: &'static str = "<c red>Unseen forces commanded your mind for a moment…!";
                                    // ignore re-force, no matter what.
                                    if command.trim().to_lowercase().starts_with("force") { continue; }
                                    // nope if 'by' self
                                    if Arc::ptr_eq(&player, &by) { continue; }
                                    // craft synthetic command.
                                    let ctx = CommandCtx {
                                        pre_pad_n: true,
                                        state: state.clone(),
                                        world: world.clone(),
                                        tx: &tx,
                                        args: &command,
                                        writer: &mut writer,
                                    };
                                    let delivery = delivery.unwrap_or_else(|| UNK_FORCE.to_string());
                                    match who {
                                        ForceTarget::All => {
                                            state = cmd::parse_and_exec(ctx).await;
                                            let prompt = player.read().await.prompt(&state).unwrap_or_else(||"#> ".into());
                                            tell_user!(&mut writer, "\n{}\n{}", delivery, prompt);
                                        },

                                        ForceTarget::Room { id } => {
                                            // void?
                                            let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                                            
                                            if !Arc::ptr_eq(&ploc, &id) { continue; }

                                            state = cmd::parse_and_exec(ctx).await;
                                            let prompt = player.read().await.prompt(&state).unwrap_or_else(||"#> ".into());
                                            tell_user!(&mut writer, "\n{}\n{}", delivery, prompt);
                                        }

                                        ForceTarget::Player { id } => {
                                            if !Arc::ptr_eq(&player, &id) { continue; }

                                            state = cmd::parse_and_exec(ctx).await;
                                            let prompt = player.read().await.prompt(&state).unwrap_or_else(||"#> ".into());
                                            tell_user!(&mut writer, "\n{}\n{}", delivery, prompt);
                                        }
                                    }
                                },
                            },
                            _ => ()
                        },
                        _ => (/* only actively playing Players get broadcasts. */)
                    },
                }
            }
        });
    }
}
