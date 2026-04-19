//! Life-thread keeps the world ticking.

use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{sync::{RwLock, mpsc}, time};

use crate::{combat::CombatantMut, identity::{IdentityMut, IdentityQuery}, io::Broadcast, room::Room, string::{Uuid, styling::maybe_plural}, thread::{SystemSignal, librarian::ENT_BP_LIBRARY, signal::{SignalChannels, SpawnType}}, world::World, player::Player};

type Battler = Arc<RwLock<dyn CombatantMut + Send + Sync>>;
/// Threshold above which we'll stop nagging the World and pre-fetch list(s) in one sweep…
pub const PARALLEL_BATTLE_CONGESTION_THRESHOLD: usize = 50;

struct BattleStage {
    vs: HashMap<String, (Battler, Battler, Arc<RwLock<Room>>)>,
}

impl Default for BattleStage {
    fn default() -> Self {
        Self { vs: HashMap::new() }
    }
}

/// Life-thread. Lives hang on in balance here!
/// 
/// Life-thread is the game's "pulse" that ticks the clocks of everything.
//TODO (It'll do) much more than that Soon™.
/// 
pub(crate) async fn life_thread((outgoing, mut incoming): (SignalChannels, mpsc::Receiver<SystemSignal>), world: Arc<RwLock<World>>) {
    let mut tick_interval = time::interval(Duration::from_millis(10));// 100Hz
    let mut battle_interval = time::interval(Duration::from_millis(1000));// 1Hz
    let mut tick = 0;
    log::info!("Life thread firing up…");
    let mut bs = BattleStage::default();
    let mut who_online: HashMap<String, tokio::sync::broadcast::Sender<Broadcast>> = HashMap::new();

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                {
                    let mut w = world.write().await;
                    w.tick().await;
                }
                tick += 1;

                #[cfg(all(debug_assertions,feature = "stresstest"))]{
                    log::trace!("tick {tick} done");
                }
            }

            _ = battle_interval.tick() => {
                if bs.vs.is_empty() { continue; }
                let mut deathrow = vec![];

                let mut active_players: HashMap<String, Arc<RwLock<Player>>> = HashMap::new();
                let mut parallel_congestion = false;
                if bs.vs.len() > PARALLEL_BATTLE_CONGESTION_THRESHOLD {
                    active_players = world.read().await.players_by_id.iter().map(|(id,p)| (id.clone(), p.clone())).collect::<HashMap<String, Arc<RwLock<Player>>>>();
                    parallel_congestion = true;
                }

                for (atk_id, (atk, vct, room)) in bs.vs.iter_mut() {
                    let resolution = punt(atk, vct, &room).await;
                    let msg = match resolution {
                        Resolution::Inconclusive => format!("...."),
                        Resolution::AtkVictory => format!("AtkV"),
                        Resolution::AtkRetreat => format!("AtkR"),
                        Resolution::VctRetreat => format!("VctR"),
                        Resolution::VctVictory => format!("VctV"),
                        Resolution::BothDead => format!("BDED"),
                    };
                    log::debug!("Round resolved as … {msg}");
                    let p = if parallel_congestion {
                        active_players.get(atk_id.as_str()).cloned()
                    } else {
                        world.read().await.players_by_id.get(atk_id.as_str()).cloned()
                    };
                    let p_exists = p.is_some();
                    if p_exists {
                        let plr = p.unwrap();
                        if let Some(tx) = who_online.get(atk_id.as_str()) {
                            tx.send(Broadcast::SystemInRoom {
                                room: room.clone(),
                                actor: plr.clone(),
                                message_actor: msg,
                                message_other: "fite!?".into()
                            }).ok();
                        }
                    } else {
                        who_online.remove(atk_id.as_str());
                    }
                    
                    if !matches!(resolution, Resolution::Inconclusive) || !p_exists {
                        deathrow.push(atk_id.clone())
                    }
                }
                for id in deathrow {
                    bs.vs.remove(&id);
                }
            }

            Some(sig) = incoming.recv() => match sig {
                SystemSignal::Shutdown => break,
                SystemSignal::Spawn {what, room_id} => spawn_something(what, &room_id, world.clone()).await,
                SystemSignal::Attack {who, victim_id} => {
                    log::debug!("ATK!?");
                    let Some(room) = who.read().await.location.upgrade() else { continue; };// skip those in the void.
                    let target_arc: Battler;
                    if let Some(ent) = room.read().await.entities.get(&victim_id) {
                        target_arc = ent.clone() as Battler;
                    } else if let Some(pvp) = room.read().await.who.get(&victim_id) {
                        let Some(pvp) = pvp.upgrade() else {
                            // TODO - direct comms to player; Broadcast - just need player thread to hand us their tx when they pop online...
                            continue;
                        };
                        target_arc = pvp.clone() as Battler;
                    } else {
                        // TODO - Broadcast to player that their target ran off...
                        log::debug!("Where'd '{victim_id}' go?");
                        continue;
                    }
                    bs.vs.insert(who.read().await.id().into(), (who.clone() as Battler, target_arc, room.clone()));
                }
                SystemSignal::PlayerLogin { who, tx } => {who_online.insert(who, tx);},
                SystemSignal::PlayerLogout { who } => {
                    bs.vs.remove(&who);
                    who_online.remove(&who);
                },
                _ => {}
            }
        }
    }

    log::info!("Lifeline checking out after {tick} tick{}. Bye now!", maybe_plural(tick));
}

/// Spawn a [Mob] or [Item] at given [Room] (by ID).
/// 
/// # Args
/// - `what` to spawn.
/// - `where` to spawn ([Room] ID).
async fn spawn_something(what: SpawnType, r#where: &str, world: Arc<RwLock<World>>) {
    match what {
        SpawnType::Mob { id } => {
            if let Some(mut mob) = (*ENT_BP_LIBRARY).read().await.get(&id) {
                *(mob.id_mut()) = mob.id().re_uuid();
                if let Some(r_arc) = world.read().await.rooms.get(r#where) {
                    r_arc.write().await.entities.insert(mob.id().into(), Arc::new(RwLock::new(mob)));
                } else {
                    log::error!("Ayy! We don't have room '{}' to spawn '{}' at!", r#where, mob.id());
                }
            } else {
                log::warn!("There's no record of '{id}' in the entity catalogue…");
            }
        }

        _ => ()
    }
}

pub enum Resolution {
    Inconclusive,
    AtkRetreat,
    VctRetreat,
    AtkVictory,
    VctVictory,
    BothDead,
}

/// Fite!
async fn punt(atk: &mut Battler, vct: &mut Battler, room: &Arc<RwLock<Room>>) -> Resolution {
    let mut a = atk.write().await;
    let mut v = vct.write().await;

    let v_ded = v.take_dmg(a.dmg());
    let a_ded = if v_ded {
        // potential last-breath counter before falling over...
        //a.take_dmg(v.dmg());
        false
    } else {
        a.take_dmg(v.dmg())
    };
    let v_flee = if !v_ded {
        // check potential fleeing state
        false
    } else { false };
    let a_flee = if !a_ded {
        // check potential fleeing state
        false
    } else { false };

    match (a_ded, v_ded, a_flee, v_flee) {
        (true, true,..) => Resolution::BothDead,
        (true, false,..) => Resolution::VctVictory,
        (false, true,..) => Resolution::AtkVictory,
        (_,_, true,..) => Resolution::AtkRetreat,
        (_,_,_,true) => Resolution::VctRetreat,
        _ => Resolution::Inconclusive
    }
}
