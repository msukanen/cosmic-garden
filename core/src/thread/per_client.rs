//! Per-client threading.

use std::{net::SocketAddr, sync::Arc};

use tokio::{io::{AsyncBufReadExt, BufReader}, net::TcpStream, sync::{RwLock, broadcast}};

use crate::{cmd::{self, CommandCtx}, r#const::{GREETING, PROMPT_LOGIN}, identity::IdentityQuery, io::{Broadcast, ClientState, ForceTarget}, reprompt_playing_user, string::{prompt::PromptType, sanitize::Sanitizer}, tell_user, thread::{SystemSignal, signal::SignalChannels}, world::World};
pub(crate) struct PerClientData {
    pub socket: TcpStream,
    pub addr: SocketAddr,
    pub system_ch: SignalChannels,
    pub world: Arc<RwLock<World>>,
    pub tx: broadcast::Sender<Broadcast>,
    pub rx: broadcast::Receiver<Broadcast>,
}

pub(crate) async fn per_client_thread( mut pcd: PerClientData ) {
    // Split the socket into a reader and a writer.
    let (reader, mut writer) = pcd.socket.into_split();
    // ...and wrap the raw reader in bufreader.
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Send a welcome message to the new client.
    let (greeting, login_prompt) = {
        let w = pcd.world.read().await;
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
            let mut w = pcd.world.write().await;
            if let Some(p) = w.players_by_sockaddr.remove(&pcd.addr) {
                // drop the named mapping here as it's not needed for logout.
                let lock = p.read().await;
                pcd.system_ch.game_tx.send(SystemSignal::PlayerLogout { who: lock.id().to_string() }).ok();
                let (id, name) = 
                    (lock.id().to_string(), lock.name.clone());
                w.players_by_id.remove(lock.id());
                if !abrupt_dc {
                    tell_user!(&mut writer, "\n<c cyan>Goodbye {}! See you soon again!</c>\n", lock.title());
                    log::trace!("Clean exit by '{id}'");
                }
                drop(lock);
                pcd.system_ch.janitor_tx.send(SystemSignal::PlayerNeedsSaving(p, id)).ok();
                log::trace!("Player '{name}' added to logout queue.");
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
                    log::info!("Client {} disconnected abruptly.", pcd.addr);
                    if state.is_in_game() {
                        abrupt_dc = true;
                        state = ClientState::Logout;
                        continue;
                    }
                    break; // not in game, cut the line, wipe the floors and take a break.
                }

                state = state.handle(&mut writer, pcd.world.clone(), &pcd.addr, &pcd.tx, &pcd.system_ch, &line.trim().sanitize()).await;
            },

            // --- Second Branch: Receive broadcast messages from other clients/system itself…
            result = pcd.rx.recv() => match state.clone() {
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
                        }

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
                        }

                        Broadcast::Logout { from, who } => {
                            let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                            if Arc::ptr_eq(&from, &ploc) {
                                tell_user!(&mut writer, "\n<c cyan>{}</c> vanishes into the mists…\n", who);
                                reprompt_playing_user!(writer, state);
                            }
                        }

                        Broadcast::System { rooms, message, from } => {
                            let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                            if let Some(sender) = from {
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
                        }

                        Broadcast::BiSignal { to, from, who, message_to, message_from, message_who } => {
                            // am I the 'who'?
                            if Arc::ptr_eq(&player, &who) {
                                tell_user!(&mut writer, "\n{}\n", message_who);
                                reprompt_playing_user!(writer, state);
                                continue;
                            }
                            // just skip if in the void
                            let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                            // at either 'to' or 'from'?
                            if !Arc::ptr_eq(&to, &ploc) && !Arc::ptr_eq(&from, &ploc) { continue; }

                            let msg = if Arc::ptr_eq(&to, &ploc) { &message_to } else { &message_from };
                            if !msg.is_empty() {
                                tell_user!(&mut writer, "\n{}\n", msg);
                                reprompt_playing_user!(writer, state);
                            }
                        }

                        Broadcast::SystemInRoom { room, actor, message_actor, message_other } => {
                            let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                            if !Arc::ptr_eq(&room, &ploc) { continue; }// not there
                            if Arc::ptr_eq(&player, &actor) {
                                tell_user!(&mut writer, "\n{}\n", message_actor);
                            } else {
                                tell_user!(&mut writer, "\n{}\n", message_other);
                            }
                            reprompt_playing_user!(writer, state);
                        }

                        Broadcast::SystemInRoomAt { room, atk, vct, message_atk, message_other, message_vct } => {
                            let Some(ploc) = player.read().await.location.upgrade() else { continue; };
                            if !Arc::ptr_eq(&room, &ploc) { continue; }// not there
                            tell_user!(&mut writer, "\n{}\n",
                                if Arc::ptr_eq(&player, &atk) {
                                    message_atk
                                } else if Arc::ptr_eq(&player, &vct) {
                                    message_vct
                                } else {
                                    message_other
                                });
                            reprompt_playing_user!(writer, state);
                        }

                        Broadcast::Force { command, who, by, delivery } => {
                            static UNK_FORCE: &'static str = "<c red>Unseen forces commanded your mind for a moment…!";
                            // ignore re-force, no matter what.
                            if command.trim().to_lowercase().starts_with("force") { continue; }
                            // nope if 'by' self
                            if let Some(by) = &by {
                                if Arc::ptr_eq(&player, &by) { continue; }
                            }
                            // craft synthetic command.
                            let ctx = CommandCtx {
                                pre_pad_n: true,
                                state: state.clone(),
                                world: pcd.world.clone(),
                                system: &pcd.system_ch,
                                tx: &pcd.tx,
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
                        }

                        Broadcast::Shutdown => {
                            tell_user!(&mut writer, "\n<c red>---[ SERVER SHUTTING DOWN ]---</c>\n");
                            state = ClientState::Logout
                        }

                        Broadcast::Message { to, message } => {
                            if !Arc::ptr_eq(&player, &to) { continue; }// not for us
                            tell_user!(&mut writer, "\n{}\n", message);
                            reprompt_playing_user!(writer, state);
                        }
                    },
                    _ => ()
                },
                _ => (/* only actively playing Players get broadcasts. */)
            },
        }
    }
    
    log::debug!("Client checking out.");
}
