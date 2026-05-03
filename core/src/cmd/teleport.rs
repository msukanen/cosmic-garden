//! Tellyportaling.

use std::sync::Arc;

use async_trait::async_trait;
use either::Either;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx, look::LookCommand}, combat::Combatant, err_tell_user, identity::{IdentityQuery, MachineIdentity}, io::Broadcast, mob::core::Entity, player::Player, room::Room, roomloc_or_bust, show_help_if_needed, tell_user, tell_user_unk, translocate, util::access::Accessor, validate_access, world::World};

pub struct TeleportCommand;

enum TeleType {
    Player(Arc<RwLock<Player>>),
    Entity(Arc<RwLock<Entity>>),
    Room(Arc<RwLock<Room>>),
}

#[async_trait]
impl Command for TeleportCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, true_builder);
        show_help_if_needed!(ctx, "u teleport");
        let (p_id, is_ghost, loc) = {
            let p = plr.read().await;
            let p_id = p.id().to_string();
            let is_ghost = p.config.is_ghost;
            let loc = roomloc_or_bust!(plr);
            (p_id, is_ghost, loc)
        };
        // let mut args = ctx.args.split_once(' ').unwrap_or_else(||(ctx.args, ""));

        // loop {
        // // what to port?
        // match args.0 {
        //     "self"|"me" => {
        //         if args.1.is_empty() {
        //             tell_user!(ctx.writer, "Right, right - teleport - but where to?\n");
        //             return;
        //         }
        //         let w = ctx.world.read().await;
        //         let target_arc = match w.get_room_by_id(args.1) {
        //             Some(target_arc) => target_arc.clone(),
        //             _ => match w.players_by_id.get(args.1) {
        //                 Some(pl_arc) => {
        //                     let p = pl_arc.read().await;
        //                     let Some(ploc) = p.location.upgrade() else {
        //                         log::warn!("Teleport destination player '{}' in void.", args.1);
        //                         tell_user!(ctx.writer, "Found a player matching that ID, but they're in the void - no can do…\n");
        //                         return;
        //                     };
        //                     ploc.clone()
        //                 },
        //                 _ => {
        //                     log::debug!("Teleport fail: destination '{}' was neither Room or Player.", args.1);
        //                     tell_user!(ctx.writer, "A slight typo there? I can't locate a room nor any player with that ID…\n");
        //                     return;
        //                 }
        //             }
        //         };
        //         log::debug!("Teleport translocating to '{}'", args.1);
        //         translocate!(plr, plr.read().await.location.upgrade().unwrap(), target_arc);
        //         drop(w);
        //         if !is_ghost {
        //             ctx.out.broadcast.send(Broadcast::System {
        //                 rooms: vec![target_arc.clone()],
        //                 from: plr.clone().into(),
        //                 message: format!("<c red>Something startling materializes in the field of vicinity…!</c>"),
        //             }).ok();
        //         }
        //         LookCommand.exec({ctx.args = "";ctx}).await;
        //         return;
        //     },

        //     other => {
        //         if args.1.is_empty() {
        //             // send self to other
        //             args = ("me", args.0);
        //             continue;
        //         }
                
        //         // ok, we have two different arcs to figure out - is 'other' a player or room and if 'args.1' is a player or room…
        //         let fst = try_resolve_to_something(other, ctx.world.clone()).await;
        //         let snd = try_resolve_to_something(args.1, ctx.world.clone()).await;
        //         match (fst, snd) {
        //             (Some(Either::Right(p_arc)), Some(Either::Left(r_arc))) => {
        //                 let Some(p_loc) = p_arc.read().await.location.upgrade() else {
        //                     tell_user!(ctx.writer, "That didn't work out too well. Player '{}' was and remains the void.\n", other);
        //                     return;
        //                 };
        //                 translocate!(p_arc.clone(), p_loc.clone(), r_arc.clone());
        //                 ctx.out.broadcast.send({
        //                     let name = p_arc.read().await.title().to_string();
        //                     Broadcast::BiSignal {
        //                     to: r_arc.clone(),
        //                     from: p_loc.clone(),
        //                     who: p_arc.clone(),
        //                     message_to: format!("Very surprised looking <c cyan>{name}</c> materializes in your vicinity…"),
        //                     message_from: format!("Suddenly the mists swallow <c cyan>{name}</c>!"),
        //                     message_who: format!("Ut-oh, the forces unseen have transported you across space and time!"),
        //                 }}).ok();
        //                 log::info!("Admin '{admin_id}' port '{other}' to room '{}'.", args.1);
        //                 tell_user!(ctx.writer, "Bon voyage, '{}'… Room '{}' awaits.\n", other, args.1);
        //             },
        //             (Some(Either::Right(p_arc_target)), Some(Either::Right(p_arc_dest))) => {
        //                 let Some(r_target) = p_arc_target.read().await.location.upgrade() else {
        //                     tell_user!(ctx.writer, "That didn't work out too well. Player '{}' was and remains the void.\n", other);
        //                     return;
        //                 };
        //                 let Some(r_dest) = p_arc_dest.read().await.location.upgrade() else {
        //                     tell_user!(ctx.writer, "Nope, '{}' is in the void. Sending '{}' there would be bad juju.\n", args.1, other);
        //                     return;
        //                 };
        //                 translocate!(p_arc_target.clone(), r_target.clone(), r_dest.clone());
        //                 ctx.out.broadcast.send({
        //                     let name = p_arc_target.read().await.title().to_string();
        //                     Broadcast::BiSignal {
        //                     to: r_dest.clone(),
        //                     from: r_target.clone(),
        //                     who: p_arc_target.clone(),
        //                     message_to: format!("Very surprised looking <c cyan>{name}</c> materializes in your vicinity…"),
        //                     message_from: format!("Suddenly the mists swallow <c cyan>{name}</c>!"),
        //                     message_who: format!("Ut-oh, the forces unseen have transported you across space and time!"),
        //                 }}).ok();
        //                 log::info!("Admin '{admin_id}' port '{other}' to player '{}'.", args.1);
        //                 tell_user!(ctx.writer, "'{}' has been sent to meet with '{}'.\n", other, args.1);
        //             },
        //             (Some(Either::Left(_)),_) => {
        //                 tell_user!(ctx.writer, "Warping spacetime and moving <c yellow>rooms</c>? How about, uh, nope?\n");
        //             },
        //             (_, None) => {
        //                 tell_user!(ctx.writer, "No matter how I read this, I can't locate '{}'.\n", args.1);
        //             },
        //             (None, _) => {
        //                 tell_user!(ctx.writer, "'{}' doesn't seem to exist, whatever it should be…\n", other);
        //             }
        //         }
        //         return ;
        //     }
        // }
        // }
        let (what, wher) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        match what {
            "me"|"self" => if wher.is_empty() {
                err_tell_user!(ctx.writer, "You move swiftly nowhere — happens when you forget to mention destination…\n");
            } else {
                match try_resolve_to_something(wher, &ctx.world, &loc).await {
                    None => err_tell_user!(ctx.writer, "Whatever or wherever '{}' might be, you have no clue.\n", wher),
                    Some(found) => { translocate(ctx, plr.clone(), TeleType::Player(plr.clone()), found).await }
                }
            }

            _ => {
                let what = match try_resolve_to_something(what, &ctx.world, &loc).await {
                    None => err_tell_user!(ctx.writer, "Ok, but… whatever '{}' is, or where, you have no clue.\n", what),
                    Some(found) => found
                };
                if wher.is_empty() {
                    translocate(ctx, plr, what, TeleType::Room(loc)).await;
                } else {
                    let wher = match try_resolve_to_something(wher, &ctx.world, &loc).await {
                        None => err_tell_user!(ctx.writer, "Shucks, whatever '{}' is or where, you have no clue.\n", wher),
                        Some(found) => found
                    };
                    translocate(ctx, plr, what, wher).await;
                }
            }
        }
    }
}

/// Resolve `id` to either Room or Player, or neither.
async fn try_resolve_to_something(id: &str, world: &Arc<RwLock<World>>, loc: &Arc<RwLock<Room>>) -> Option<TeleType> {
    let w = world.read().await;

    if let Some(room) = w.get_room_by_id(id) {
        Some(TeleType::Room(room))
    }
    else if let Some(plr) = w.players_by_id.get(id) {
        Some(TeleType::Player(plr.clone()))
    }
    else if let Some(ent) = loc.read().await.entities.get(&id.as_m_id()) {
        Some(TeleType::Entity(ent.clone()))
    } else {
        None
    }
}

async fn translocate(ctx: &mut CommandCtx<'_>, initiator: Arc<RwLock<Player>>, what: TeleType, wher: TeleType ) {
    let Some(ini_loc) = initiator.read().await.location().upgrade().clone() else {
        log::error!("Initiator '{}' in the void!", initiator.read().await.id());
        err_tell_user!(ctx.writer, "<c red>[ERR]</c> <c yellow>Not happening!</c>\n <c blue>*</c> Not going to teleport anything into the void with you…\n <c blue>*</c> Move to Garden space first.\n");
    };

    let (vct_id, vct_title, tgt) =
    match (what, wher) {
        (TeleType::Entity(_), TeleType::Entity(_)) => err_tell_user!(ctx.writer, "Bonking heads is one thing, but to occupy same space with teleport? Too cruel…\n"),
        
        (TeleType::Entity(e), tgt) => {
            let tgt = match tgt {
                TeleType::Player(tgt) => {
                    let Some(wher_arc) = tgt.read().await.location.upgrade() else {
                        err_tell_user!(ctx.writer, "Target player found, but…! They're in the void. Not going to teleport anything there!\n");
                    };
                    wher_arc
                }

                TeleType::Room(arc) => arc,
                _ => unreachable!("Handled already.")
            };
            let m_id = e.read().await.id().as_m_id();
            
            translocate!(ent e, m_id, ini_loc, tgt);
            
            let e_lock = e.read().await;
            let t_lock = tgt.read().await;
            (
                e_lock.id().to_string(),
                e_lock.title().to_string(),
                format!("to {} ({})", t_lock.title().to_string(), t_lock.id().to_string())
            )
        }
        
        (TeleType::Player(vct), TeleType::Entity(_)) => {
            // erase last goto as there's no backtracking teleport…
            vct.write().await.last_goto = None;

            let (v_id, maybe_fake_loc) = {
                let lock = vct.read().await;
                let v_id = lock.id().to_string();
                let v_loc = lock.location().upgrade().unwrap_or_else(|| ini_loc.clone());
                (v_id, v_loc)
            };
            
            translocate!(vct, v_id, maybe_fake_loc, ini_loc);
            
            let v_lock = vct.read().await;
            (
                v_id,
                v_lock.title().to_string(),
                "right here".into()
            )
        }

        (TeleType::Player(vct), tgt) => {
            // erase last goto as there's no backtracking teleport…
            vct.write().await.last_goto = None;

            let tgt = match tgt {
                TeleType::Room(arc) => arc,
                TeleType::Player(oth) => {
                    let Some(arc) = oth.read().await.location().upgrade().clone() else {
                        err_tell_user!(ctx.writer, "Target player '{}' in the void. Not sending '{}' there!\n", oth.read().await.id(), vct.read().await.id());
                    };
                    arc
                },
                _ => unreachable!("Handled already.")
            };
            let v_id = vct.read().await.id().to_string();
            let maybe_fake_loc = match vct.read().await.location().upgrade() {
                None => ini_loc.clone(),
                Some(arc) => arc.clone()
            };
            
            translocate!(vct, v_id, maybe_fake_loc, tgt);
            
            let v_lock = vct.read().await;
            let t_lock = tgt.read().await;
            (
                v_id,
                v_lock.title().to_string(),
                format!("to {} ({})", t_lock.title().to_string(), t_lock.id().to_string())
            )
        }
        _ => err_tell_user!(ctx.writer, "Nope — not going to warp spacetime by teleporting *rooms*!\n")
    };

    tell_user!(ctx.writer, "Resolution…: {} ({}) translocated {}.\n", vct_title, vct_id, tgt)
}
