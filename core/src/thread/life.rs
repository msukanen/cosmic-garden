//! Life-thread keeps the world ticking.

use std::{collections::HashMap, sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{sync::RwLock, time};

use crate::{combat::CombatantMut, identity::{IdentityMut, IdentityQuery}, io::Broadcast, player::Player, room::Room, string::{Uuid, styling::maybe_plural}, thread::{SystemSignal, librarian::ENT_BP_LIBRARY, signal::{SigReceiver, SignalSenderChannels, SpawnType}}, translocate, world::World};

#[cfg(test)]
#[macro_export]
macro_rules! get_operational_mock_life {
    ($ch:ident, $w:ident) => {
        tokio::spawn( crate::thread::life(($ch.out.clone(), $ch.recv.life), $w.clone()))
    };
}

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

trait IdentityQueryLite {
    async fn id(&self) -> String;
}

impl IdentityQueryLite for Battler {
    async fn id(&self) -> String {
        self.read().await.id().into()
    }
}

lazy_static! {
    /// How many "general" ticks there is in a second…
    pub(crate) static ref TICKS_PER_SECOND: Arc<RwLock<u64>> = Arc::new(RwLock::new(100));
}

/// Life-thread. Lives hang on in balance here!
/// 
/// Life-thread is the game's "pulse" that ticks the clocks of everything.
//TODO (It'll do) much more than that Soon™.
/// 
pub(crate) async fn life((out, mut incoming): (SignalSenderChannels, SigReceiver), world: Arc<RwLock<World>>) {
    let mut tick_interval = time::interval(Duration::from_millis(1_000 / *(TICKS_PER_SECOND.read().await)));// 100Hz
    let mut battle_interval = time::interval(Duration::from_millis(1_000));// 1Hz
    let mut tick = 0;
    log::info!("Life thread firing up…");
    let mut bs = BattleStage::default();
    let mut who_online: HashMap<String, String> = HashMap::new();// id, title

    loop {
        tokio::select! {
            //
            // The "World Clock".
            //
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

            //
            // Battle handler tick.
            //
            _ = battle_interval.tick() => {
                if bs.vs.is_empty() { continue; }
                let mut deathrow = vec![];

                let mut active_players: HashMap<String, Arc<RwLock<Player>>> = HashMap::new();
                let mut parallel_congestion = false;
                if bs.vs.len() > PARALLEL_BATTLE_CONGESTION_THRESHOLD {
                    active_players = world.read().await.players_by_id.iter().map(|(id,p)| (id.clone(), p.clone())).collect::<HashMap<String, Arc<RwLock<Player>>>>();
                    parallel_congestion = true;
                }

                // Resolve the ongoing combats for this tick…
                for (atk_id, (atk, vct, room)) in bs.vs.iter_mut() {
                    let resolution = punt(atk, vct, &room).await;
                    let message_actor = match resolution {
                        Resolution::Inconclusive => format!("You hit {}.", vct.id().await),
                        Resolution::AtkVictory => format!("You're victorious against {}!", vct.id().await),
                        Resolution::AtkRetreat => format!("Better run while you can…"),
                        Resolution::VctRetreat => format!("Hey, {} is running away!", vct.id().await),
                        Resolution::VctVictory => format!("Ouch… *you faint*"),
                        Resolution::BothDead => format!("You fall… flat on your face. R.I.P."),
                    };
                    let message_other = match resolution {
                        Resolution::Inconclusive => format!("{atk_id} hits {}.", vct.id().await),
                        Resolution::AtkVictory => format!("{atk_id} has slain {}!", vct.id().await),
                        Resolution::AtkRetreat => format!("{atk_id} runs away for their dear life…"),
                        Resolution::VctRetreat => format!("Huh, {} is running away…", vct.id().await),
                        Resolution::VctVictory => format!("{atk_id} collapses due numerous, too numerous, wounds."),
                        Resolution::BothDead => format!("Unexpected… Both {atk_id} and {} fall over at the same time, either dead or exhausted.", vct.id().await),
                    };
                    //log::debug!("Round for {atk_id} vs {} resolved as … {message_actor}", vct.id().await);
                    let p = if parallel_congestion {
                        active_players.get(atk_id.as_str()).cloned()
                    } else {
                        world.read().await.players_by_id.get(atk_id.as_str()).cloned()
                    };
                    let p_exists = p.is_some();
                    if p_exists {
                        let plr = p.unwrap();
                        who_online.get(atk_id.as_str()).and_then(|_| {
                            log::debug!("Broadcasting round resolution…");
                            out.broadcast.send(Broadcast::SystemInRoom {
                                room: room.clone(),
                                actor: plr.clone(),
                                message_actor,
                                message_other
                            }).ok()
                        });
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

            //
            // System signals.
            //
            Some(sig) = incoming.recv() => match sig {
                SystemSignal::Shutdown => break,

                // send item spawns to Librarian
                SystemSignal::Spawn {what: SpawnType::Item {id}, room_id} => {
                    #[cfg(test)]{log::debug!("Routing SystemSignal::Spawn{{Item}} to Librarian.");}
                    out.librarian.send(SystemSignal::Spawn { what: SpawnType::Item { id }, room_id }).ok();
                },
                SystemSignal::Spawn {what, room_id} => spawn_something(what, &room_id, world.clone()).await,
                SystemSignal::Attack {who, victim_id} => {
                    // log::debug!("ATK!?");
                    let Some(room) = who.read().await.location.upgrade() else { continue; };// skip those in the void.
                    let target_arc: Battler;
                    if let Some(ent) = room.read().await.entities.get(&victim_id) {
                        target_arc = ent.clone() as Battler;
                    } else if let Some(pvp) = room.read().await.who.get(&victim_id) {
                        let atk_id = who.read().await.id().to_string();
                        let Some(pvp) = pvp.upgrade() else {
                            // they're gone...? (or never were there to begin with)
                            who_online.get(&atk_id).and_then(|_|
                                out.broadcast.send(Broadcast::Message { to: who.clone(), message: "They're not here…".into() }).ok()
                            );
                            continue;
                        };
                        let vct_name = pvp.read().await.title().to_string();
                        target_arc = pvp.clone() as Battler;
                        who_online.get(&atk_id).and_then(|atk_name|
                            out.broadcast.send(Broadcast::SystemInRoomAt {
                                room: room.clone(),
                                atk: who.clone(),
                                vct: pvp.clone(),
                                message_atk: format!("You attack {vct_name}!"),
                                message_vct: format!("{atk_name} attacks you!"),
                                message_other: format!("{atk_name} attacks {vct_name}!")
                            }).ok()
                        );
                    } else {
                        // TODO - Broadcast to player that their target ran off...
                        log::debug!("Where'd '{victim_id}' go?");
                        continue;
                    }
                    bs.vs.insert(who.read().await.id().into(), (who.clone() as Battler, target_arc, room.clone()));
                }
                SystemSignal::PlayerLogin { id, title } => {who_online.insert(id, title);},
                SystemSignal::PlayerLogout { id } => {
                    bs.vs.remove(&id);
                    who_online.remove(&id);
                },

                //
                // Public transportation (or denial of such thereof).
                //
                SystemSignal::WantTransportFromTo { who, from, to, via } => {
                    let who_id = who.read().await.id().to_string();
                    if bs.vs.contains_key(&who_id) {
                        // in combat, transit denied
                        who_online.get(&who_id).and_then(|_| {
                            out.broadcast.send(Broadcast::Message { to: who.clone(), message: "You're in middle of combat! Try <c yellow>flee</c> first…".into() }).ok()
                        });
                        continue;
                    }

                    log::debug!("Transport request from {who_id} from {} to {}", from.read().await.id(), to.read().await.id());
                    translocate!(who, from, to);

                    let mut plr = who.write().await;
                    let origin_id = from.read().await.id().to_string();
                    plr.last_goto = Some((via.into(), Arc::downgrade(&from)));
                    log::debug!("Last goto: {} from <{origin_id}>", plr.last_goto.as_ref().unwrap().0);
                    drop(plr);

                    if let Err(e) = out.broadcast.send(Broadcast::Force { silent: true, command: "look".into(), who: crate::io::ForceTarget::Player { id: who }, by: None, delivery: None }) {
                        log::error!("Communications blackout?! {e:?}");
                    };
                },
                SystemSignal::AbortBattleNow { who } => {
                    bs.vs.remove(&who);
                }
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
async fn punt(atk: &mut Battler, vct: &mut Battler, _room: &Arc<RwLock<Room>>) -> Resolution {
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
