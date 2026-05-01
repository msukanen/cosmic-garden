//! Tellyportaling.

use std::sync::Arc;

use async_trait::async_trait;
use either::Either;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx, look::LookCommand}, identity::{IdentityQuery, MachineIdentity}, io::Broadcast, player::Player, player_or_bust, room::Room, tell_user, tell_user_unk, translocate, util::access::Accessor, world::World};

pub struct TeleportCommand;

#[async_trait]
impl Command for TeleportCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        if !plr.read().await.access.is_true_builder() {
            tell_user_unk!(ctx.writer);
            return;
        }
        
        if ctx.args.is_empty() {
            tell_user!(ctx.writer, "<c green>Usage:</c> teleport <dest-id>\n       teleport <id> <dest-id>\n");
            return;
        }

        let (admin_id, is_ghost) = {
            let p = plr.read().await;
            let admin_id = p.id().to_string();
            let is_ghost = p.config.is_ghost;
            (admin_id, is_ghost)
        };
        let mut args = ctx.args.split_once(' ').unwrap_or_else(||(ctx.args, ""));

        loop {
        // what to port?
        match args.0 {
            "self"|"me" => {
                if args.1.is_empty() {
                    tell_user!(ctx.writer, "Right, right - teleport - but where to?\n");
                    return;
                }
                let w = ctx.world.read().await;
                let target_arc = match w.rooms.get(&args.1.as_m_id()) {
                    Some(target_arc) => target_arc.clone(),
                    _ => match w.players_by_id.get(args.1) {
                        Some(pl_arc) => {
                            let p = pl_arc.read().await;
                            let Some(ploc) = p.location.upgrade() else {
                                log::warn!("Teleport destination player '{}' in void.", args.1);
                                tell_user!(ctx.writer, "Found a player matching that ID, but they're in the void - no can do…\n");
                                return;
                            };
                            ploc.clone()
                        },
                        _ => {
                            log::debug!("Teleport fail: destination '{}' was neither Room or Player.", args.1);
                            tell_user!(ctx.writer, "A slight typo there? I can't locate a room nor any player with that ID…\n");
                            return;
                        }
                    }
                };
                log::debug!("Teleport translocating to '{}'", args.1);
                translocate!(plr, plr.read().await.location.upgrade().unwrap(), target_arc);
                drop(w);
                if !is_ghost {
                    ctx.out.broadcast.send(Broadcast::System {
                        rooms: vec![target_arc.clone()],
                        from: plr.clone().into(),
                        message: format!("<c red>Something startling materializes in the field of vicinity…!</c>"),
                    }).ok();
                }
                LookCommand.exec({ctx.args = "";ctx}).await;
                return;
            },

            other => {
                if args.1.is_empty() {
                    // send self to other
                    args = ("me", args.0);
                    continue;
                }
                
                // ok, we have two different arcs to figure out - is 'other' a player or room and if 'args.1' is a player or room…
                let fst = try_resolve_to_something(other, ctx.world.clone()).await;
                let snd = try_resolve_to_something(args.1, ctx.world.clone()).await;
                match (fst, snd) {
                    (Some(Either::Right(p_arc)), Some(Either::Left(r_arc))) => {
                        let Some(p_loc) = p_arc.read().await.location.upgrade() else {
                            tell_user!(ctx.writer, "That didn't work out too well. Player '{}' was and remains the void.\n", other);
                            return;
                        };
                        translocate!(p_arc.clone(), p_loc.clone(), r_arc.clone());
                        ctx.out.broadcast.send({
                            let name = p_arc.read().await.title().to_string();
                            Broadcast::BiSignal {
                            to: r_arc.clone(),
                            from: p_loc.clone(),
                            who: p_arc.clone(),
                            message_to: format!("Very surprised looking <c cyan>{name}</c> materializes in your vicinity…"),
                            message_from: format!("Suddenly the mists swallow <c cyan>{name}</c>!"),
                            message_who: format!("Ut-oh, the forces unseen have transported you across space and time!"),
                        }}).ok();
                        log::info!("Admin '{admin_id}' port '{other}' to room '{}'.", args.1);
                        tell_user!(ctx.writer, "Bon voyage, '{}'… Room '{}' awaits.\n", other, args.1);
                    },
                    (Some(Either::Right(p_arc_target)), Some(Either::Right(p_arc_dest))) => {
                        let Some(r_target) = p_arc_target.read().await.location.upgrade() else {
                            tell_user!(ctx.writer, "That didn't work out too well. Player '{}' was and remains the void.\n", other);
                            return;
                        };
                        let Some(r_dest) = p_arc_dest.read().await.location.upgrade() else {
                            tell_user!(ctx.writer, "Nope, '{}' is in the void. Sending '{}' there would be bad juju.\n", args.1, other);
                            return;
                        };
                        translocate!(p_arc_target.clone(), r_target.clone(), r_dest.clone());
                        ctx.out.broadcast.send({
                            let name = p_arc_target.read().await.title().to_string();
                            Broadcast::BiSignal {
                            to: r_dest.clone(),
                            from: r_target.clone(),
                            who: p_arc_target.clone(),
                            message_to: format!("Very surprised looking <c cyan>{name}</c> materializes in your vicinity…"),
                            message_from: format!("Suddenly the mists swallow <c cyan>{name}</c>!"),
                            message_who: format!("Ut-oh, the forces unseen have transported you across space and time!"),
                        }}).ok();
                        log::info!("Admin '{admin_id}' port '{other}' to player '{}'.", args.1);
                        tell_user!(ctx.writer, "'{}' has been sent to meet with '{}'.\n", other, args.1);
                    },
                    (Some(Either::Left(_)),_) => {
                        tell_user!(ctx.writer, "Warping spacetime and moving <c yellow>rooms</c>? How about, uh, nope?\n");
                    },
                    (_, None) => {
                        tell_user!(ctx.writer, "No matter how I read this, I can't locate '{}'.\n", args.1);
                    },
                    (None, _) => {
                        tell_user!(ctx.writer, "'{}' doesn't seem to exist, whatever it should be…\n", other);
                    }
                }
                return ;
            }
        }
        }
    }
}

/// Resolve `id` to either Room or Player, or neither.
async fn try_resolve_to_something(id: &str, world: Arc<RwLock<World>>) -> Option<Either<Arc<RwLock<Room>>, Arc<RwLock<Player>>>> {
    let w = world.read().await;

    if let Some(room) = w.rooms.get(&id.as_m_id()) {
        return Some(Either::Left(room.clone()));
    }

    if let Some(plarc) = w.players_by_id.get(id) {
        return Some(Either::Right(plarc.clone()));
    }

    None
}
