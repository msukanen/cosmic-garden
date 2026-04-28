//! Life-thread keeps the world ticking.

use std::{collections::HashMap, sync::{Arc, Weak}, time::Duration};

use nohash_hasher::BuildNoHashHasher;
use tokio::{sync::{RwLock, mpsc, oneshot}, time};

use crate::{combat::{Battler, CombatantMut}, identity::{IdentityMut, IdentityQuery}, io::Broadcast, item::Item, mob::{StatValue, core::Entity}, room::Room, string::{DescribableMut, Uuid, styling::maybe_plural}, thread::{SystemSignal, add_item_to_lnf, signal::{SigReceiver, SignalSenderChannels, SpawnType}}, traits::Reflector, translocate, util::approx::ApproxI32, world::World};

#[cfg(test)]
#[macro_export]
macro_rules! get_operational_mock_life {
    ($ch:ident, $w:ident) => {
        tokio::spawn( crate::thread::life(($ch.out.clone(), $ch.recv.life), $w.clone()))
    };
}

// Threshold above which we'll stop nagging the World and pre-fetch list(s) in one sweep…
//pub const PARALLEL_BATTLE_CONGESTION_THRESHOLD: usize = 50;

pub(crate) type BattlerKey = usize;
#[derive(Clone)]
struct BattlerRec {
    combatant: Battler,
    title: Arc<str>,
}

impl BattlerRec {
    async fn loot_pinata(&self, world: &Arc<RwLock<World>>) {
        let mut lock = self.combatant.write().await;
        if let Some(room) = lock.location().upgrade() {
            let mut c_inv = Item::Corpse(lock.inventory().deep_reflect());
            let c_id = lock.id().to_string();
            let c_title = lock.title().to_string();
            drop(lock);
            c_inv.set_desc(&format!("Corpse of '{}'", c_title));
            {
                let mut lock = room.write().await;
                lock.entities.remove(&c_id);
                if let Err(e) = lock.try_insert(c_inv) {
                    // well shucks, room full…
                    add_item_to_lnf(e).await;
                }
            }
            world.write().await.entities.remove(&c_id);
        } else {
            log::warn!("No piñataing '{}' in the void…", self.title);
        }
    }
}

type BattleMap = HashMap<BattlerKey, (BattlerRec, Arc<RwLock<Room>>), BuildNoHashHasher<BattlerKey>>;
type AggroMap = HashMap<BattlerKey, Vec<BattlerKey>, BuildNoHashHasher<BattlerKey>>;

/// Battle stage — the place for all battles.
struct BattleStage {
    active: BattleMap,
    atk: AggroMap,
    vct: AggroMap,
}

impl Default for BattleStage {
    fn default() -> Self {
        Self {
            active: BattleMap::default(),
            atk: AggroMap::default(),
            vct: AggroMap::default(),
        }
    }
}

impl BattleStage {
    async fn remove(&mut self, battle_key: BattlerKey) {
        // first the vct …
        if let Some(atks) = self.vct.remove(&battle_key) {
            for a_key in atks {
                // remove the ded from aggro list
                if let Some(tgt) = self.atk.get_mut(&a_key) {
                    if let Some(pos) = tgt.iter().position(|&x| x == battle_key) {
                        tgt.swap_remove(pos);
                    }
                    // anybody out there?
                    if tgt.is_empty() {
                        self.atk.remove(&a_key);
                        if let Some((rec, _)) = self.active.get_mut(&a_key) {
                            rec.combatant.write().await.alter_brain_freeze(false);
                        }
                        self.active.remove(&a_key);
                    }
                }
            }
        }

        // …and then who was beating the vct …
        if let Some(tgts) = self.atk.remove(&battle_key) {
            for t_key in tgts {
                if let Some(atks) = self.vct.get_mut(&t_key) {
                    if let Some(pos) = atks.iter().position(|&x| x == battle_key) {
                        atks.swap_remove(pos);
                    }
                }
            }
        }

        // …and final purge.
        if let Some((rec, _)) = self.active.get_mut(&battle_key) {
            rec.combatant.write().await.alter_brain_freeze(false);
        }
        self.active.remove(&battle_key);
    }

    async fn remove_b(&mut self, battler: &Battler) {
        let key = lock2key!(arc &battler);
        self.remove(key).await;
    }
}

/// Combat resolutions.
#[derive(Debug, Clone)]
pub enum Resolution {
    Inconclusive { atk_dmg: StatValue, vct_dmg: StatValue },
    AtkRetreat,
    VctRetreat,
    AtkVictory { atk_dmg: StatValue },
    VctVictory  { vct_dmg: StatValue },
    BothDead,
}

/// Query seconds-as-ticks.
pub async fn sec_as_ticks(sec: u32, tick_type: TickType, out: &SignalSenderChannels) -> usize {
    let (otx,orx) = tokio::sync::oneshot::channel::<u32>();
    out.life.send(SystemSignal::SecToTicks { sec, tick_type, out: otx }).ok();
    if let Ok(sat) = orx.await {
        sat as usize
    } else {
        log::warn!("Life thread too busy to tell us current tick rate… Assuming default of 100Hz.");
        (sec * 100) as usize
    }
}

enum LifeWorkerSignal {
    BattleOk { atk: BattlerRec, vct: BattlerRec, room: Arc<RwLock<Room>> },
    BattleFail { atk: Battler, vct: Battler },
    BattleMsg { atk: BattlerRec, vct: BattlerRec, resolution: Resolution },
    Shutdown,
}

pub enum TickType {
    Core,
    Battle,
}

/// Life-thread. Lives hang on in balance here!
/// 
/// Life-thread is the game's "pulse" that ticks the clocks of everything.
//TODO (It'll do) much more than that Soon™.
/// 
pub(crate) async fn life((out, mut incoming): (SignalSenderChannels, SigReceiver), world: Arc<RwLock<World>>) {
    log::info!("Intervals, intervals…");
    let mut tick_interval_hz: u64 = 100;
    let mut battle_interval_hz: u64 = 50;

    let mut tick_interval = time::interval(Duration::from_millis(1000/ tick_interval_hz));
    let mut battle_interval = time::interval(Duration::from_millis(1000 / battle_interval_hz));
    let mut tick = 0;
    let mut bs = BattleStage::default();
    let (worker_out, mut worker_rx) = mpsc::unbounded_channel::<LifeWorkerSignal>();
    let (reporter_out, mut reporter_rx) = mpsc::unbounded_channel::<LifeWorkerSignal>();
    let battle_reporter = tokio::spawn({
        let out = out.broadcast.clone();
        async move {
            while let Some(impact) = reporter_rx.recv().await {
                match impact {
                    LifeWorkerSignal::BattleMsg { atk, vct, resolution } => {
                        let Some(room_arc) = atk.combatant.read().await.location().upgrade() else {
                            log::warn!("Reporter noted that attacker '{}' isn't tethered to reality…", atk.combatant.read().await.id());
                            continue;
                        };
                        // craft message(s) for participant(s)…
                        // TODO: fine-grain lethality coeff for dmg vs max_hp.
                        //       C_lethal = dmg / HP_max
                        //       ...and figure out some verbal noise to represent it...
                        //          C < 0.01 : "scratches", "grazes", "pokes"?
                        //          0.01 < C < 0.1 : "hits", "strikes", "lashes"?
                        //          0.1 < C < 0.3 : "smashes", "rends", "tears"?
                        //          0.3 < C < 0.6 : "shatters", "crushes", "mutilates"?
                        //          C > 0.6 : "obliterates", "erases", "annihilates"?
                        //       ...something like that, depending on dmg deliver type...
                        let message_atk = match resolution {
                            Resolution::Inconclusive{atk_dmg, vct_dmg} => format!("You hit {} for {} dmg #vct_dmg({}).",
                                vct.title,
                                atk_dmg.approx_i32(),
                                vct_dmg.approx_i32()
                            ),
                            Resolution::AtkVictory{atk_dmg} => format!("You're victorious against {} with last hit of {} dmg!", vct.title, atk_dmg.approx_i32()),
                            Resolution::AtkRetreat => format!("Better run while you can…"),
                            Resolution::VctRetreat => format!("Hey, {} is running away!", vct.title),
                            Resolution::VctVictory{vct_dmg} => format!("Ouch… {} dmg in the face – *you faint*", vct_dmg.approx_i32()),
                            Resolution::BothDead => format!("You fall… flat on your face. R.I.P."),
                        };
                        let message_other = match resolution {
                            Resolution::Inconclusive{..} => format!("{} hits {}.", atk.title, vct.title),
                            Resolution::AtkVictory{..} => format!("{} has slain {}!", atk.title, vct.title),
                            Resolution::AtkRetreat => format!("{} runs away for their dear life…", atk.title),
                            Resolution::VctRetreat => format!("Huh, {} is running away…", vct.title),
                            Resolution::VctVictory{..} => format!("{} collapses due numerous, too numerous, wounds.", atk.title),
                            Resolution::BothDead => format!("Unexpected… Both {} and {} fall over at the same time, either dead or exhausted.", atk.title, vct.title),
                        };
                        let message_vct = match resolution {
                            Resolution::Inconclusive{atk_dmg, vct_dmg} => format!("{} hits you for {} dmg #vct_dmg({}).",
                                atk.title,
                                atk_dmg.approx_i32(),
                                vct_dmg.approx_i32()
                            ),
                            Resolution::AtkVictory{atk_dmg} => format!("Ouch… {} dmg in the face – *you faint*", atk_dmg.approx_i32()),
                            Resolution::AtkRetreat => format!("Hey, {} is running away! Yay?", atk.title),
                            Resolution::VctRetreat => format!("Better run while you can…"),
                            Resolution::BothDead => format!("You fall… flat on your face. R.I.P."),
                            Resolution::VctVictory{vct_dmg} => format!("You're victorious against {} with last hit of {} dmg!", atk.title, vct_dmg.approx_i32()),
                        };
                        //-----------------
                        out.send(Broadcast::BattleMessage3 {
                            room: room_arc.clone(),
                            atk: atk.combatant.clone(),
                            vct: vct.combatant.clone(),
                            message_atk,
                            message_other,
                            message_vct,
                        }).ok();
                    }

                    LifeWorkerSignal::Shutdown => break,
                    _ => ()
                }
            }
        }
    });

    log::info!("Life thread firing up…");
    #[cfg(all(test, feature = "stresstest"))]
    let mut spawn_count: usize = usize::MAX;
    #[cfg(all(test, feature = "stresstest"))]
    let mut spawn_out: Option<tokio::sync::oneshot::Sender<()>> = None;

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
                if tick % 1000 == 0 {
                    log::debug!("{tick} ticks…");
                }
            }

            //
            // Battle handler tick.
            //
            _ = battle_interval.tick() => {
                if bs.active.is_empty() { continue; }
                let mut deathrow: Vec<usize> = vec![];

                // navigate the aggro swamp…
                for (a_key, vcts) in &bs.atk {
                    if let Some((atk, room_arc)) = bs.active.get(&a_key) {
                        if let Some(v_key) = vcts.get(0) {
                            if let Some((vct, _)) = bs.active.get(&v_key) {
                                let resolution = punt(atk.combatant.clone(), vct.combatant.clone(), &room_arc).await;
                                reporter_out.send(LifeWorkerSignal::BattleMsg {
                                    atk: atk.clone(),
                                    vct: vct.clone(),
                                    resolution: resolution.clone()
                                }).ok();
                                log::debug!("resolution = {resolution:?}");
                                // TODO deal with possible loot drops if not Resolution::Inconclusive or XyzRetreat:
                                match resolution {
                                    Resolution::AtkRetreat => { deathrow.push(*a_key);},
                                    Resolution::AtkVictory {..} => {
                                        deathrow.push(*v_key);
                                        vct.loot_pinata(&world).await;
                                    }
                                    Resolution::VctRetreat => { deathrow.push(*v_key);},
                                    Resolution::VctVictory {..} => {
                                        deathrow.push(*a_key);
                                        atk.loot_pinata(&world).await;
                                    },
                                    Resolution::BothDead => {
                                        deathrow.push(*a_key);
                                        deathrow.push(*v_key);
                                        vct.loot_pinata(&world).await;
                                        atk.loot_pinata(&world).await;
                                    },
                                    Resolution::Inconclusive {..} => ()
                                }
                            } else {
                                // no v_key in battle stage? … weird.
                                deathrow.push(*v_key);
                            }
                        } else {
                            // was no opponents, kthxbye.
                            deathrow.push(*a_key);
                        }
                    } else {
                        // not in active list? WTF?
                        deathrow.push(*a_key);
                    }

                }

                for d in deathrow {
                    bs.remove(d).await;
                }
            }

            //
            // System signals.
            //
            Some(sig) = incoming.recv() => match sig {
                // System is going down, now.
                SystemSignal::Shutdown => {
                    reporter_out.send(LifeWorkerSignal::Shutdown).ok();
                    break
                },

                // Re-send item spawns to Librarian.
                SystemSignal::Spawn {what: SpawnType::Item {id}, room_id} => {
                    log::warn!("Routing SystemSignal::Spawn{{Item}} to Librarian. FIX the source, should go straight to Librarian and not via Life.");
                    out.librarian.send(SystemSignal::Spawn { what: SpawnType::Item { id }, room_id }).ok();
                }

                // Spawn some [Entity].
                SystemSignal::Spawn {what, room_id} => {
                    #[cfg(all(test, feature = "stresstest"))]
                    {
                        spawn_count = spawn_count.saturating_sub(1);
                        if spawn_count == 0 {
                            if let Some(out) = spawn_out {
                                out.send(()).ok();
                            }
                            spawn_out = None;
                        }
                        if spawn_count % 10_000 == 0 {
                            log::info!("Spawns to go: {spawn_count}");
                        }
                    }
                    spawn_something(&out, what, &room_id, &world).await
                }

                // Attack! From e.g. AttackCommand from player to start a fight.
                SystemSignal::Attack {atk_arc, vct_arc} => {
                    // We might be busy, let a worker handle the initial hurdle.
                    tokio::spawn({
                        let sig = worker_out.clone();
                        //let sys = out.clone();
                        async move {
                            let (a_loc, a_title) = {
                                let a = atk_arc.read().await;
                                (a.location().upgrade(), a.title().to_string())
                            };
                            let v_title = vct_arc.read().await.title().to_string();
                            if let Some(room) = a_loc {
                                #[cfg(test)]{log::trace!("Battle-check: OK \"{a_title}\"|{} vs \"{v_title}\"|{}", atk_arc.read().await.id(), vct_arc.read().await.id());}
                                
                                let atk = BattlerRec {
                                    combatant: atk_arc,
                                    title: Arc::from(a_title),
                                };
                                let vct = BattlerRec {
                                    combatant: vct_arc,
                                    title: Arc::from(v_title),
                                };
                                sig.send(LifeWorkerSignal::BattleOk { atk, vct, room: room.clone() }).ok();
                            } else {
                                log::error!("Cannot initiate fight in the void!");
                                sig.send(LifeWorkerSignal::BattleFail { atk: atk_arc, vct: vct_arc }).ok();
                            }
                        }
                    });
                }

                // Player loggeed out.
                SystemSignal::PlayerLogout { player } => bs.remove_b(&(player as Battler)).await,

                //
                // Public transportation (or denial of such thereof).
                //
                SystemSignal::WantTransportFromTo { who, from, to, via } => {
                    let who_key = Weak::as_ptr(&Arc::downgrade(&who)) as *const() as BattlerKey;
                    let who_id = who.read().await.id().to_string();
                    if bs.active.contains_key(&who_key) {
                        out.broadcast.send(Broadcast::Message { to: who.clone(), message: "You're in middle of combat! Try <c yellow>flee</c> first…".into() }).ok();
                        continue;
                    }

                    log::trace!("Transport request from {who_id} from {} to {}", from.read().await.id(), to.read().await.id());
                    translocate!(who, from, to);

                    let mut plr = who.write().await;
                    let origin_id = from.read().await.id().to_string();
                    plr.last_goto = Some((via.into(), Arc::downgrade(&from)));
                    log::trace!("Last goto: {} from <{origin_id}>", plr.last_goto.as_ref().unwrap().0);
                    drop(plr);

                    if let Err(_) = out.broadcast.send(Broadcast::Force {silent: true, command: "look".into(), who: crate::io::ForceTarget::Player { id: who }, by: None, delivery: None }) {
                        log::error!("Communications blackout?!");
                    };
                }

                // Abort battle for `who`.
                SystemSignal::AbortBattleNow { who } => bs.remove_b(&who).await,
                
                // Count how many ticks `sec` is currently.
                SystemSignal::SecToTicks { sec, tick_type, out } => {
                    out.send(sec * match tick_type {
                        TickType::Core => tick_interval_hz,
                        TickType::Battle => battle_interval_hz,
                    } as u32).ok();
                }

                // Alter tick timers.
                SystemSignal::AlterTickRate { tick_type, duration} => {
                    match tick_type {
                        TickType::Core => {
                            tick_interval_hz = (1.0 / duration.as_secs_f32()) as u64;
                            tick_interval = tokio::time::interval(duration);
                            tick_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
                        }

                        TickType::Battle => {
                            battle_interval_hz = (1.0 / duration.as_secs_f32()) as u64;
                            battle_interval = tokio::time::interval(duration);
                            battle_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
                        }
                    }
                }
                
                #[cfg(all(test, feature = "stresstest"))]
                SystemSignal::CountSpawns { num, out } => {
                    spawn_count = num;
                    spawn_out = Some(out);
                    log::warn!("Preparing to count spawns down from {num}…");
                }

                _ => {}
            },

            //
            // Listen to potential workers.
            //
            Some(sig) = worker_rx.recv() => match sig {
                LifeWorkerSignal::BattleOk { atk, vct, room } => {
                    let a_key = lock2key!(arc &atk.combatant);
                    let v_key = lock2key!(arc &vct.combatant);
                    atk.combatant.write().await.alter_brain_freeze(true);
                    vct.combatant.write().await.alter_brain_freeze(true);
                    bs.active.insert(a_key, (atk, room.clone()));
                    bs.active.insert(v_key, (vct, room.clone()));
                    if let Some(a) = bs.atk.get_mut(&a_key) {
                        if !a.contains(&v_key) {
                            a.push(v_key);
                        }
                    } else {
                        log::trace!("New attacker: {a_key}");
                        bs.atk.insert(a_key, vec![v_key]);
                    }
                    if let Some(v) = bs.vct.get_mut(&v_key) {
                        if !v.contains(&a_key) {
                            v.push(a_key);
                        }
                    } else {
                        log::trace!("New victim: {v_key}");
                        bs.vct.insert(v_key, vec![a_key]);
                    }
                    log::debug!("LifeworkerSignal::BattleOk!");
                }

                LifeWorkerSignal::BattleFail { atk, vct } => {
                    log::debug!("LifeworkerSignal::BattleFail");
                    // attempt purge, just in case.
                    let a_key = Weak::as_ptr(&Arc::downgrade(&atk)) as *const() as BattlerKey;
                    let v_key = Weak::as_ptr(&Arc::downgrade(&vct)) as *const() as BattlerKey;
                    bs.remove(a_key).await;
                    bs.remove(v_key).await;
                }

                _ => ()
            }
        }
    }

    battle_reporter.await.ok();
    log::info!("Lifeline checking out after {tick} tick{}. Bye now!", maybe_plural(tick));
}

/// Spawn a [Mob] or [Item] at given [Room] (by ID).
/// 
/// # Args
/// - `what` to spawn.
/// - `where` to spawn ([Room] ID).
async fn spawn_something(out: &SignalSenderChannels, what: SpawnType, r_id: &str, world: &Arc<RwLock<World>>) {
    match &what {
        SpawnType::Mob { id } => {
            let w = world.read().await;
            if let Some(r_arc) = w.rooms.get(r_id) {
                let r_arc = r_arc.clone();
                drop(w);
                direct_spawn_something(out, what, &r_arc, world).await
            } else {
                log::error!("Ayy! We don't have room '{}' to spawn '{id}' at!", r_id);
            }
        }

        _ => ()
    }
}

/// Spawn a [Mob] or [Item] at given [Room].
/// 
/// # Args
/// - `what` to spawn.
/// - `where` to spawn ([Room] ID).
async fn direct_spawn_something(out: &SignalSenderChannels, what: SpawnType, r_arc: &Arc<RwLock<Room>>, world: &Arc<RwLock<World>>)
{
    match what {
        SpawnType::Mob { id } => {
            let (oneshot, recv) = oneshot::channel::<Option<Entity>>();
            if let Ok(_) = out.librarian.send(SystemSignal::EntityBlueprintReq { id: id.clone(), out: oneshot }) {
                if let Ok(reply) = recv.await {
                    if let Some(mut mob) = reply {
                        *(mob.id_mut()) = mob.id().re_uuid();
                        let mob_id = mob.id().to_string();
                        mob.set_location(&r_arc);
                        let mob_arc = Arc::new(RwLock::new(mob));
                        {
                            let mut w = world.write().await;
                            // tell the world 1st…
                            w.entities.insert(mob_id.clone(), Arc::downgrade(&mob_arc));
                            // …then the room itself.
                            r_arc.write().await.entities.insert(mob_id.clone(), mob_arc);
                        }// drop 'w' now…
                        log::trace!("Life has spawned '{mob_id}' at '{}'", r_arc.read().await.id());
                    } else {
                        log::warn!("There's no record of '{id}' in the entity catalogue…");
                    }
                } else {
                    log::warn!("Librarian is too busy to check her catalogues for '{id}'… Oh well.");
                }
            } else {
                log::warn!("Message system congestion. No can do…");
            }
        }

        _ => ()
    }
}

/// Fite!
async fn punt(atk: Battler, vct: Battler, _room: &Arc<RwLock<Room>>) -> Resolution {
    let mut a = atk.write().await;
    let mut v = vct.write().await;

    let atk_dmg = a.dmg();
    let v_ded = v.take_dmg(atk_dmg);
    let (a_ded, vct_dmg) = if v_ded {
        // potential last-breath counter before falling over...
        //a.take_dmg(v.dmg());
        (false, 0.0)
    } else {
        let vct_dmg = v.dmg();
        (a.take_dmg(vct_dmg), vct_dmg)
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
        (true, false,..) => Resolution::VctVictory {vct_dmg},
        (false, true,..) => Resolution::AtkVictory {atk_dmg},
        (_,_, true,..) => Resolution::AtkRetreat,
        (_,_,_,true) => Resolution::VctRetreat,
        _ => Resolution::Inconclusive {atk_dmg, vct_dmg}
    }
}

#[cfg(test)]
mod life_tests {
    use std::{io::Cursor, sync::Arc};

    use crate::{stabilize_threads, cmd::look::LookCommand, combat::{Battler, CombatantMut}, r#const::SMALL_ITEM, get_operational_mock_janitor, get_operational_mock_librarian, identity::IdentityQuery, item::{Item, container::Storage, ownership::Owner, weapon::{WeaponSize, WeaponSpec}}, thread::{SystemSignal, life::BattlerRec, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn goblin_ocean() {
        #[cfg(feature = "stresstest")]
        const MILLION_GOBBOS: usize = 1_000_000;
        #[cfg(not(feature = "stresstest"))]
        const MILLION_GOBBOS: usize = 1_000;// just 1,000 if not stresstesting...

        let (w,c,_,j) = get_operational_mock_world().await;
        get_operational_mock_janitor!(c,w,j.0);
        get_operational_mock_life!(c,w);
        get_operational_mock_librarian!(c,w);
        let c = c.out;// we don't need the c.recv part anymore here…
        stabilize_threads!();
        let (otx,orx) = tokio::sync::oneshot::channel::<()>();
        c.life.send(SystemSignal::CountSpawns { num: MILLION_GOBBOS, out: otx }).ok();
        for _ in 1..=MILLION_GOBBOS {
            c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into()}).ok();
        }
        // let the dust settle…
        let _ = orx.await;
        log::debug!("--terminated--");
    }

    #[tokio::test]
    async fn loot_pinata() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state, p),j) = get_operational_mock_world().await;
        let _ = get_operational_mock_janitor!(c,w,j.0);
        let _ = get_operational_mock_life!(c,w);
        let _ = get_operational_mock_librarian!(c,w);
        let c = c.out;
        stabilize_threads!();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into()}, room_id: "r-1".into()}).ok();
        stabilize_threads!(100);
        log::debug!("Stabilized…");
        let lock = w.read().await;
        if let Some(r1) = lock.rooms.get("r-1") {
            let r1 = r1.clone();
            drop(lock);
            log::debug!("Dropped world lock…");
            let lock = r1.read().await;
            let mut e_id = String::new();
            for k in lock.entities.keys() {
                if k.starts_with("goblin") {
                    e_id = k.clone();
                    break;
                }
            }
            if e_id.is_empty() {
                panic!("Oi! No lil gobbo found!");
            }
            log::debug!("Found lil gobbo…");
            if let Some(e) = lock.entities.get(&e_id) {
                let e = e.clone();
                drop(lock);
                log::debug!("Dropped room lock…");
                let spec = WeaponSpec {
                    id: "stabber".into(),
                    name: "Gobbo Stabber".into(),
                    desc: "A stabber for gobbos!".into(),
                    owner: Owner::no_one(),
                    size: SMALL_ITEM,
                    weapon_size: WeaponSize::Small,
                    base_dmg: 1.9,
                };
                let mut lock = e.write().await;
                lock.inventory().try_insert(Item::Weapon(spec)).ok();
                log::debug!("Gobbo has a stabber nao!");
                let erec = BattlerRec {
                    combatant: e.clone() as Battler,
                    title: Arc::from(lock.title().to_string())
                };
                drop(lock);
                state = ctx!(state, LookCommand, "", s,c,w,p);
                log::debug!("Lootage…?");
                erec.loot_pinata(&w).await;
                log::debug!("Got loots…!");
            } else {
                panic!("Where did the gobbo go?! It was right here!");
            }
        } else {
            panic!("Ok, where did the room vanish?");
        }
        stabilize_threads!(100);
        p.write().await.access = Access::Builder;
        p.write().await.config.show_id = true;
        let _ = ctx!(state, LookCommand, "", s,c,w,p);
        log::debug!("--terminated--");
    }
}
