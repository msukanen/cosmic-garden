//! Tellyportaling.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx}, combat::Combatant, err_tell_user, identity::{IdentityQuery, MachineIdentity}, mob::core::Entity, player::Player, room::Room, roomloc_or_bust, show_help_if_needed, tell_user, translocate, validate_access, world::World};

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
            let loc = roomloc_or_bust!(ctx, plr);
            (p_id, is_ghost, loc)
        };

        let (what, wher) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        match what {
            // Teleport self.
            "me"|"self" => if wher.is_empty() {
                err_tell_user!(ctx.writer, "You move swiftly nowhere — happens when you forget to mention destination…\n");
            } else {
                match try_resolve_to_something(wher, &ctx.world, &loc).await {
                    None => err_tell_user!(ctx.writer, "Whatever or wherever '{}' might be, you have no clue.\n", wher),
                    Some(found) => { translocate(ctx, plr.clone(), TeleType::Player(plr.clone()), found).await }
                }
            }

            // Teleport everyone and everything in the room with you.
            "all" => if wher.is_empty() {
                err_tell_user!(ctx.writer, "Ok… they move real swift nowhere at all. So, mind tell \"where to\"?\n");
            } else {
                match try_resolve_to_something(wher, &ctx.world, &loc).await {
                    None => err_tell_user!(ctx.writer, "Whatever or wherever '{}' might be, you have no clue.\n", wher),
                    Some(found) => {
                        let t_loc = match found {
                            TeleType::Entity(_) => err_tell_user!(ctx.writer, "Well… hauling everyone to some specific entity? Maybe not today.\n"),
                            TeleType::Player(p) => {
                                if let Some(p_loc) = p.read().await.location().upgrade() {
                                    if Arc::ptr_eq(&p_loc, &loc) {
                                        err_tell_user!(ctx.writer, "Sure, easy. They're already there, as in… right here.\n");
                                    } else {
                                        p_loc.clone()
                                    }
                                } else {
                                    err_tell_user!(ctx.writer, "Ut-oh, that player is in the void?! Nope, not going to haul everyone *there*!\n");
                                }
                            }
                            TeleType::Room(r) => r.clone()
                        };
                        // acquire targets and start warping the spacetime…
                    }
                }
            }

            // …or teleport something somewhere else (or yank to where you are…)
            _ => {
                let what = match try_resolve_to_something(what, &ctx.world, &loc).await {
                    None => err_tell_user!(ctx.writer, "Ok, but… whatever '{}' is, or where, you have no clue.\n", what),
                    Some(found) => found
                };
                if wher.is_empty() {
                    // Yank.
                    translocate(ctx, plr, what, TeleType::Room(loc)).await;
                } else {
                    let wher = match try_resolve_to_something(wher, &ctx.world, &loc).await {
                        None => err_tell_user!(ctx.writer, "Shucks, whatever '{}' is or where, you have no clue.\n", wher),
                        Some(found) => found
                    };
                    // Port.
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
    }
    else if let Some(ent) = world.read().await.entities.get(&id.as_m_id()) {
        if let Some(arc) = ent.upgrade() {
            Some(TeleType::Entity(arc.clone()))
        } else {
            None
        }
    }
    // serious fallbacks:
    else {
        // see room entity-by-entity
        for e_arc in loc.read().await.entities.values() {
            let lock = e_arc.read().await;
            if lock.id().starts_with(id) {
                return Some(TeleType::Entity(e_arc.clone()));
            }
        }
        // worst case scenario: world…
        for e in world.read().await.entities.values() {
            if let Some(e_arc) = e.upgrade() {
                let lock = e_arc.read().await;
                if lock.id().starts_with(id) {
                    return Some(TeleType::Entity(e_arc.clone()));
                }
            }
        }
        log::debug!("No {id}({}) found anywhere!", id.as_m_id());
        None
    }
}

/// Translocate an Entity or Player to another Player or Entity.
async fn translocate(ctx: &mut CommandCtx<'_>, initiator: Arc<RwLock<Player>>, what: TeleType, wher: TeleType ) {
    // Nail the initiator down during translocate! We'll avoid any potential spacetime warps with it.
    // … and keep the lock immutable even though it's a .write lock. We're not writing anything, just
    //   keeping them still over the course of translocate's course.
    let Ok(ini_lock) = initiator.try_write() else {
        log::warn!("translocate deadlock dodged.");
        err_tell_user!(ctx.writer, "Yikes! Better try this later, spacetime too turbulent…\n");
    };

    let Some(ini_loc) = ini_lock.location().upgrade().clone() else {
        log::error!("Initiator '{}' in the void!", initiator.read().await.id());
        err_tell_user!(ctx.writer, "<c red>[ERR]</c> <c yellow>Not happening!</c>\n <c blue>*</c> Not going to teleport anything into the void with you…\n <c blue>*</c> Move to Garden space first.\n");
    };

    let (vct_id, vct_title, tgt) =
    match (what, wher) {
        (TeleType::Entity(e1), TeleType::Entity(e2)) =>
            if Arc::ptr_eq(&e1, &e2) {
                err_tell_user!(ctx.writer, "Schrödinger's {}? Lets not try that…\n", e1.read().await.title());
            } else {
                err_tell_user!(ctx.writer, "Bonking heads is one thing, but to occupy same space with teleport? Too cruel…\n")
            }
        
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
        
        (TeleType::Player(vct), TeleType::Entity(ent)) => {
            // erase last goto as there's no backtracking teleport…
            vct.write().await.last_goto = None;

            let (v_id, maybe_fake_loc) = {
                let lock = vct.read().await;
                let v_id = lock.id().to_string();
                let v_loc = lock.location().upgrade().unwrap_or_else(|| ini_loc.clone());
                (v_id, v_loc)
            };
            
            let t_loc = ent.read().await.location().upgrade().unwrap_or_else(|| ini_loc.clone());
            translocate!(vct, v_id, maybe_fake_loc, t_loc);
            
            let v_lock = vct.read().await;
            (
                v_id,
                v_lock.title().to_string(),
                if Arc::ptr_eq(&ini_loc, &t_loc) { "right here".into() } else {t_loc.read().await.id().into()}
            )
        }

        (TeleType::Player(vct), tgt) => {
            // erase last goto as there's no backtracking teleport…

            let tgt = match tgt {
                TeleType::Room(arc) => arc,
                TeleType::Player(oth) => {
                    if Arc::ptr_eq(&vct, &oth) {
                        err_tell_user!(ctx.writer, "But… they're already at themselves? No point teleporting them to where they already are.\n");
                    }
                    let Some(arc) = oth.read().await.location().upgrade().clone() else {
                        err_tell_user!(ctx.writer, "Target player '{}' in the void. Not sending '{}' there!\n", oth.read().await.id(), vct.read().await.id());
                    };
                    arc
                },
                _ => unreachable!("Handled already.")
            };
            let (maybe_fake_loc, v_id) = {
                let mut lock = vct.write().await;
                lock.last_goto = None;
                let v_id = lock.id().to_string();
                (match lock.location().upgrade() {
                    None => {
                        log::warn!("Target {v_id} was in the void… pulling into reality.");
                        ini_loc.clone()
                    },
                    Some(arc) => arc.clone()
                }, v_id)
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

        // Room + Entity combination *summons* the entity from designated room to you, if possible.
        (TeleType::Room(from), TeleType::Entity(ent)) => {
            let mut here = ini_loc.write().await;
            let mut there = from.write().await;
            let lock = ent.write().await;
            let ent_id = lock.id().to_string();
            let ent_title = lock.title().to_string();
            let m_id = ent_id.as_m_id();
            drop(lock);
            there.entities.remove(&m_id);
            here.entities.insert(m_id, ent);
            (
                ent_id,
                ent_title,
                "to right here".into()
            )
        }
        _ => err_tell_user!(ctx.writer, "Nope — not going to warp spacetime by teleporting *rooms*!\n")
    };

    tell_user!(ctx.writer, "Resolution…: {} ({}) translocated {}.\n", vct_title, vct_id, tgt)
}

#[cfg(test)]
mod cmd_teleport_tests {
    use std::{io::Cursor, sync::{Arc, Weak}};

    use tokio::sync::RwLock;

    use crate::{cmd::{look::LookCommand, teleport::TeleportCommand}, ctx, get_operational_mock_librarian, get_operational_mock_life, identity::{IdentityMut, IdentityQuery, uniq::Uuid}, player::Player, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn teleport_self() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),_) = get_operational_mock_world().await;
        get_operational_mock_life!(c,w);
        get_operational_mock_librarian!(c,w);
        stabilize_threads!();
        let c = c.out;
        state = ctx!(sup state,TeleportCommand,"",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Player { event_host: false, builder: true };
        state = ctx!(sup state,TeleportCommand,"",s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        state = ctx!(sup state,TeleportCommand,"",s,c,w,|out:&str| out.contains("Usage"));
        
        let r2 = w.read().await.get_room_by_id("r-2").unwrap().clone();
        let mut p2 = Player::default();
        let p2_id = "p2".to_string();
        p2.set_id(&p2_id, false).ok();
        p2.set_title("Player#2");
        p2.set_location(&r2).await;
        let p2 = Arc::new(RwLock::new(p2));
        r2.write().await.who.insert(p2_id.clone(), Arc::downgrade(&p2));
        w.write().await.players_by_id.insert(p2_id.clone(), p2);

        state = ctx!(sup state,TeleportCommand,"p3",s,c,w,|out:&str| out.contains("whatever 'p3'"));
        
        let r1 = w.read().await.get_room_by_id("r-1").unwrap().clone();
        // void attempt
        p.write().await.location = Weak::new();
        state = ctx!(sup state,TeleportCommand,"p2",s,c,w,|out:&str| out.contains("in the void"));
        p.write().await.location = Arc::downgrade(&r1);
        state = ctx!(sup state,TeleportCommand,"p2",s,c,w,|out:&str| out.contains("Resolution"));

        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: crate::room::RoomPayload::Id("r-2".to_string()), reply: None }).ok();
        stabilize_threads!(10);
        state = ctx!(sup state,TeleportCommand,"p2 boglin",s,c,w,|out:&str| out.contains("Shucks"));
        state = ctx!(state,TeleportCommand,"p2 goblin",s,c,w);
        // room + ent transports ent from designated room (instead of trying to warp spacetime by transporting the room…)
        state = ctx!(state,TeleportCommand,"r-2 goblin",s,c,w);

        stabilize_threads!(100);
    }
}
