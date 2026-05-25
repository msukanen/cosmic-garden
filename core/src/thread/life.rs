//! Life-thread keeps the world ticking.

use std::{collections::HashMap, sync::Arc};
use once_cell::sync::OnceCell;

use lazy_static::lazy_static;
use nohash_hasher::BuildNoHashHasher;
use tokio::{sync::{mpsc, oneshot}, time::{Duration, Instant, MissedTickBehavior, interval}};

use crate::{
    combat::{Battler, BattlerRec, CombatantMut, Resolution, punt, register_ok_battle}, identity::{IdentityMut, IdentityQuery, MachineId, MachineIdentity, uniq::Uuid}, io::Broadcast, item::container::storage::Storage, mob::{EntityArc, core::Entity}, room::{RoomArc, RoomPayload, locking::Exit}, string::styling::maybe_plural, thread::{SystemSignal, signal::{SigReceiver, SignalSenderChannels, SpawnType}}, translocate, util::{approx::ApproxI32, direction::Direction}, world::WorldArc
};

lazy_static! {
    pub static ref CORE_HZ: OnceCell<u8> = OnceCell::new();
    pub static ref BATTLE_HZ: OnceCell<u8> = OnceCell::new();
}

#[cfg(test)]
#[macro_export]
macro_rules! get_operational_mock_life {
    ($ch:ident, $w:ident) => {
        tokio::spawn( crate::thread::life(($ch.out.clone(), $ch.recv.life), $w.clone(), (
            *crate::thread::life::CORE_HZ.get().unwrap(),
            *crate::thread::life::BATTLE_HZ.get().unwrap()
        )))
    };
}

type BattleMap = HashMap<MachineId, (BattlerRec, RoomArc), BuildNoHashHasher<MachineId>>;
type AggroMap = HashMap<MachineId, Vec<MachineId>, BuildNoHashHasher<MachineId>>;

/// Battle stage — the place for all battles.
pub(crate) struct BattleStage {
    pub active: BattleMap,
    pub atk: AggroMap,
    pub vct: AggroMap,
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
    fn clear(&mut self) {
        self.active.clear();
        self.atk.clear();
        self.vct.clear();
    }

    async fn remove(&mut self, battle_key: MachineId) {
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

/// Query seconds-as-ticks.
pub fn sec_as_ticks(sec: u32, tick_type: TickType) -> usize {
    (sec * *(match tick_type {
        TickType::Core => CORE_HZ.get().expect("Core Hz not set?!"),
        TickType::Battle => BATTLE_HZ.get().expect("Battle Hz not set?!"),
    }) as u32) as usize
}

enum LifeWorkerSignal {
    BattleOk { atk: BattlerRec, vct: BattlerRec, room: RoomArc },
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
pub(crate) async fn life(
    (out, mut incoming): (SignalSenderChannels, SigReceiver),
    world: WorldArc,
    (core_hz, battle_hz) : (u8, u8),
) {
    const fn core_hz_ms(core_hz: u8) -> u64 { 1000/ core_hz as u64 }
    const fn battle_hz_ms(battle_hz: u8) -> u64 { 1_000/ battle_hz as u64 }

    let mut tick_interval = interval(Duration::from_millis(core_hz_ms(core_hz)));
            tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut battle_interval = interval(Duration::from_millis(battle_hz_ms(battle_hz)));
            battle_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // Battle stage & reporter for it…
    let mut bs = BattleStage::default();
    let (worker_out, mut worker_rx) = mpsc::unbounded_channel::<LifeWorkerSignal>();
    let (reporter_out, mut reporter_rx) = mpsc::unbounded_channel::<LifeWorkerSignal>();
    let battle_reporter = tokio::spawn({
        let out = out.broadcast.clone();
        static RES_BOTH_DEAD_AV: &'static str = "You fall… flat on your face. R.I.P.";
        static RES_ABORT_RWARP: &'static str = "Er, OK? What just happened… Where they go?";
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
                        let (message_atk, message_vct, message_other) = match resolution {
                            Resolution::Inconclusive{atk_dmg, vct_dmg} => {
                                match (atk_dmg, vct_dmg) {
                                    (None, None) => continue,
                                    _ => ()
                                }
                                let d_a = atk_dmg.approx_i32();
                                let d_v = vct_dmg.approx_i32();
                                (
                                    format!("You hit {} for {d_a} dmg #vct_dmg({d_v}).", vct.title),
                                    format!("{} hits you for {d_a} dmg #vct_dmg({d_v}).", atk.title),
                                    format!("{} hits {}.", atk.title, vct.title),
                                )
                            },
                            Resolution::AtkVictory{atk_dmg} => {
                                let d_a = atk_dmg.approx_i32();
                                (
                                    format!("You're victorious against {} with last hit of {d_a} dmg!", vct.title),
                                    format!("Ouch… {d_a} dmg in the face – *you faint*"),
                                    format!("{} has slain {}!", atk.title, vct.title),
                                )
                            }
                            Resolution::AtkRetreat => (
                                "Better run while you can…".into(),
                                format!("Hey, {} is running away! Yay?", atk.title),
                                format!("{} runs away for their dear life…", atk.title),
                            ),
                            Resolution::VctRetreat => (
                                format!("Hey, {} is running away!", vct.title),
                                "Better run while you can…".into(),
                                format!("Huh, {} is running away…", vct.title),
                            ),
                            Resolution::VctVictory{vct_dmg} => {
                                let d_v = vct_dmg.approx_i32();
                                (
                                    format!("Ouch… {d_v} dmg in the face – *you faint*"),
                                    format!("You're victorious against {} with last hit of {d_v} dmg!", atk.title),
                                    format!("{} collapses due numerous, too numerous, wounds.", atk.title),
                                )
                            },
                            Resolution::BothDead => (
                                RES_BOTH_DEAD_AV.into(),
                                RES_BOTH_DEAD_AV.into(),
                                format!("Unexpected… Both {} and {} fall over at the same time, either dead or exhausted.", atk.title, vct.title),
                            ),
                            Resolution::AbortDueRealityWarp => (
                                RES_ABORT_RWARP.into(),
                                RES_ABORT_RWARP.into(),
                                format!("Huh…?!"),
                            )
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
    #[cfg(test)]
    let mut spawn_count: usize = usize::MAX;
    #[cfg(test)]
    let mut spawn_out: Option<tokio::sync::oneshot::Sender<()>> = None;

    let mut tick = 0;
    #[cfg(debug_assertions)]
    let start_time = Instant::now();
    loop {
        tokio::select! {
            //
            // The "World Clock".
            //
            ci = tick_interval.tick() => {
                let tick_start = Instant::now();
                {
                    let mut w = world.write().await;
                    w.tick(tick).await;
                }
                tick += 1;
                let elapsed = tick_start.elapsed();
                #[cfg(debug_assertions)]
                let drift = Instant::now().duration_since(ci);
                if elapsed > Duration::from_millis(10) {
                    log::warn!("Slow tick: {tick} took {elapsed:?}?!");
                }
                #[cfg(debug_assertions)]{
                if tick % 1000 == 0 {
                    let total_elapsed = start_time.elapsed().as_secs_f64();
                    log::debug!("{tick} ticks… {total_elapsed:.2}s | drift {drift:?} | expected {}s", tick / 100);
                }}
            }

            //
            // Battle handler tick.
            //
            _ = battle_interval.tick() => {
                if bs.active.is_empty() { continue; }
                // "global" battle interval counter
                static mut C: usize = 1;

                #[cfg(all(test, feature = "super-verbose"))]{ log::debug!("Battle-tick… {}", unsafe {C} ); }
                let mut end_fight_for: Vec<usize> = vec![];

                // navigate the aggro swamp…
                for (a_key, vcts) in &bs.atk {
                    if let Some((atk, room_arc)) = bs.active.get(&a_key) {
                        // TODO: get 1st victim in line for now, until AoE etc. get brainstormed.
                        if let Some(v_key) = vcts.get(0) {
                            if let Some((vct, _)) = bs.active.get(&v_key) {
                                let resolution = punt(unsafe {C}, atk.combatant.clone(), vct.combatant.clone(), &room_arc).await;
                                reporter_out.send(LifeWorkerSignal::BattleMsg {
                                    atk: atk.clone(),
                                    vct: vct.clone(),
                                    resolution: resolution.clone()
                                }).ok();
                                #[cfg(all(test, feature = "super-verbose"))]{log::debug!("resolution = {resolution:?}");}
                                match resolution {
                                    Resolution::AtkRetreat => { end_fight_for.push(*a_key);},
                                    Resolution::AtkVictory {..} => {
                                        end_fight_for.push(*v_key);
                                        vct.loot_pinata(&world).await;
                                    }
                                    Resolution::VctRetreat => { end_fight_for.push(*v_key);},
                                    Resolution::VctVictory {..} => {
                                        end_fight_for.push(*a_key);
                                        atk.loot_pinata(&world).await;
                                    },
                                    Resolution::BothDead => {
                                        end_fight_for.push(*a_key);
                                        end_fight_for.push(*v_key);
                                        vct.loot_pinata(&world).await;
                                        atk.loot_pinata(&world).await;
                                    },
                                    // cannot continue — admin intervention?
                                    //  ≡ remove both.
                                    Resolution::AbortDueRealityWarp => {
                                        #[cfg(test)]{log::debug!("Reality warp - aborting combat {a_key}-vs-{v_key}!");}
                                        end_fight_for.push(*a_key);
                                        end_fight_for.push(*v_key);
                                    }
                                    Resolution::Inconclusive {..} => ()
                                }
                            } else {
                                // no v_key in battle stage? … weird.
                                end_fight_for.push(*v_key);
                            }
                        } else {
                            // was no opponents, kthxbye.
                            end_fight_for.push(*a_key);
                        }
                    } else {
                        // not in active list? WTF?
                        end_fight_for.push(*a_key);
                    }
                }

                for d in end_fight_for {
                    bs.remove(d).await;
                }

                unsafe { C += 1; }
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
                SystemSignal::Spawn { what: SpawnType::Item {id}, room, reply } => {
                    log::warn!("Routing SystemSignal::Spawn{{Item}} to Librarian. FIX the source, should go straight to Librarian and not via Life.");
                    out.librarian.send( SystemSignal::Spawn { what: SpawnType::Item { id }, room, reply }).ok();
                }

                // Spawn some [Entity].
                SystemSignal::Spawn { what, room, reply } => {
                    #[cfg(test)]
                    {
                        spawn_count = spawn_count.saturating_sub(1);
                        if spawn_count == 0 {
                            if let Some(out) = spawn_out {
                                out.send(()).ok();
                            }
                            spawn_out = None;
                        }
                        if spawn_count % 1_000 == 0 {
                            log::info!("Spawns to go: {spawn_count}");
                        }
                    }

                    let ok = spawn_something(&out, what, &room, &world).await;
                    if let Some(out) = reply {
                        out.send(ok).ok();
                    }
                }

                // Spawn `num` [Entity]s in a batch. Seldom used, but…
                SystemSignal::SpawnBatch {what, room, num, reply} => {
                    #[cfg(test)]
                    {
                        spawn_count = spawn_count.saturating_sub(1);
                        if spawn_count == 0 {
                            if let Some(out) = spawn_out {
                                out.send(()).ok();
                            }
                            spawn_out = None;
                        }
                        if spawn_count % 1_000 == 0 {
                            log::info!("Spawns to go: {spawn_count}");
                        }
                    }

                    let ok = spawn_something_batch(&out, what, num, &room, &world).await;
                    if let Some(out) = reply {
                        log::debug!("Sending {ok:?} about {num} spawns.");
                        out.send(ok).ok();
                    }
                }

                // Attack! From e.g. AttackCommand from player to initiate a fight.
                SystemSignal::Attack {atk_arc, vct_arc} => {
                    // We might be busy, let a worker handle the initial hurdle.
                    tokio::spawn({
                        let sig = worker_out.clone();
                        async move {
                            let (a_loc, a_title) = {
                                let a = atk_arc.read().await;
                                (a.location().upgrade(), a.title().to_string())
                            };
                            let v_title = vct_arc.read().await.title().to_string();

                            if let Some(room) = a_loc {
                                #[cfg(test)]{
                                    log::debug!("Battle-check: OK \"{a_title}\"|{} vs \"{v_title}\"|{}", atk_arc.read().await.id(), vct_arc.read().await.id());
                                }
                                
                                let atk = BattlerRec {
                                    combatant: atk_arc,
                                    title: std::sync::Arc::from(a_title),
                                };
                                let vct = BattlerRec {
                                    combatant: vct_arc,
                                    title: std::sync::Arc::from(v_title),
                                };
                                // Signal life that it's fine to proceed with this fight.
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
                // Public [Player] transportation (or denial of such thereof).
                //
                SystemSignal::WantTransportFromTo { who, from, to, via } => {
                    let who_key = lock2key!(arc &who);
                    if bs.active.contains_key(&who_key) {
                        log::debug!("Combat move rejected.");
                        out.broadcast.send(Broadcast::Message { to: who.clone(), message: "You're in middle of combat! Try <c yellow>flee</c> first…".into() }).ok();
                        continue;
                    }

                    log::trace!("Transport request by {} from {} to {}",
                        who.read().await.id(),
                        from.read().await.id(),
                        to.read().await.id()
                    );
                    translocate!(who, from, to);

                    let mut plr = who.write().await;
                    let origin_id = from.read().await.id().to_string();
                    match to.read().await.memory_fog() {
                        None => plr.last_goto = Some((via.into(), Arc::downgrade(&from))),
                        _ => plr.last_goto = None
                    }
                    log::trace!("Last goto: {} from <{origin_id}>", plr.last_goto.as_ref().unwrap().0);
                    drop(plr);

                    if let Err(_) = out.broadcast.send(Broadcast::Force {silent: true, command: "look".into(), who: crate::io::ForceTarget::Player { id: who }, by: None, delivery: None }) {
                        log::error!("Broadcast channel(s) out of business – communications blackout?!");
                    };
                }

                //
                // Public Entity transportation (or denial of such thereof).
                //
                SystemSignal::EntityWantTransportFromTo { who, from, to, via } => {
                    let who_key = lock2key!(arc &who);
                    if bs.active.contains_key(&who_key) {
                        continue;
                    }
                    tokio::spawn(async move { transport_entity(who, from, to, via) });
                }

                // Abort battle for `who`.
                SystemSignal::AbortBattleNow { who } => bs.remove_b(&who).await,
                SystemSignal::AbortAllBattle => bs.clear(),
                
                #[cfg(test)] SystemSignal::CountSpawns { num, out } => {
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
                LifeWorkerSignal::BattleOk { atk, vct, room } => register_ok_battle(atk,vct,room,&mut bs).await,
                LifeWorkerSignal::BattleFail { atk, vct } => {
                    // attempt purge, just in case.
                    let a_key = lock2key!(arc atk);
                    let v_key = lock2key!(arc vct);
                    bs.remove(a_key).await;
                    bs.remove(v_key).await;
                }

                // we ignore all the other LifeWorkerSignals here as they're for the workers, not for main thread.
                _ => ()
            }
        }
    }

    battle_reporter.await.ok();
    log::info!("Lifeline checking out after {tick} tick{}. Bye now!", maybe_plural(tick as i64));// `tick` most likely doesn't overflow i64…, like ever.
}

/// Spawn a [Mob] or [Item] at given [Room] (by ID).
/// 
/// # Args
/// - `what` to spawn.
/// - [`room`][RoomPayload] to spawn in.
/// - [`world`][World].
/// 
/// # Returns
/// Spawned?
async fn spawn_something(out: &SignalSenderChannels, what: SpawnType, room: &RoomPayload, world: &WorldArc) -> bool {
    match &what {
        SpawnType::Mob { id } => {
            let w = world.read().await;
            if let Some(r_arc) = w.get_room_by_m_id(room.id().await.as_m_id()) {
                let r_arc = r_arc.clone();
                drop(w);
                direct_spawn_something(out, what, 1, &r_arc, world).await
            } else {
                log::error!("Ayy! We don't have room '{}' to spawn '{id}' at!", room.id().await);
                false
            }
        },_=> false
    }
}

/// As per [spawn_something], but with `num` [Entity] at once.
/// # Args
/// - `what` to spawn.
/// - `num` of spawns.
/// - [`room`][RoomPayload] to spawn in.
/// - [`world`][World].
/// 
/// # Returns
/// Spawned?
async fn spawn_something_batch(out: &SignalSenderChannels, what: SpawnType, num: usize, room: &RoomPayload, world: &WorldArc) -> bool {
    match &what {
        SpawnType::Mob { id } => {
            let w = world.read().await;
            if let Some(r_arc) = w.get_room_by_m_id(room.id().await.as_m_id()) {
                let r_arc = r_arc.clone();
                drop(w);
                direct_spawn_something(out, what, num, &r_arc, world).await
            } else {
                log::error!("Ayy! We don't have room '{}' to spawn '{id}' at!", room.id().await);
                false
            }
        },_=> false
    }
}

/// Spawn a [Mob] or [Item] at given [Room].
/// 
/// # Args
/// - `what` to spawn.
/// - `where` to spawn ([Room] ID).
/// 
/// # Returns
/// Spawned?
async fn direct_spawn_something(out: &SignalSenderChannels, what: SpawnType, num: usize, r_arc: &RoomArc, world: &WorldArc) -> bool
{
    static mut C: usize = 0;
    match what {
        SpawnType::Mob { id } => {
            let (oneshot, recv) = oneshot::channel::<Option<Entity>>();
            if let Ok(_) = out.librarian.send(SystemSignal::EntityBlueprintReq { id: id.clone(), out: oneshot }) {
                if let Ok(reply) = recv.await {
                    if let Some(bp_mob) = reply {
                        let mut w = world.write().await;
                        let mut r = r_arc.write().await;
                        
                        // spawn the horde! Or less of entities…
                        for _ in 0..num {
                            let mut mob = bp_mob.clone();
                            mob.set_id(&bp_mob.id().re_uuid(), true).ok();
                            *(mob.location_mut()) = Arc::downgrade(&r_arc);
                            let mob_arc: EntityArc = mob.into();
                            // set the tick-ID
                            let mob_m_id = {
                                let mut mw = mob_arc.write().await;
                                mw.set_tick_id(&mob_arc)
                            };
                            // tell the world 1st…
                            w.add_entity(mob_m_id, Arc::downgrade(&mob_arc));
                            // …then the room itself.
                            r.add_entity(mob_m_id, mob_arc);
                            unsafe { C += 1; }
                        }
                        log::debug!("Spawned {num} entit{}.", if num==1{"y"} else {"ies"});
                        return true;
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

    false
}

/// Attempt to transport [Entity] `from` `to` `via`.
async fn transport_entity(who: EntityArc, from: RoomArc, to: RoomArc, via: Direction) {
    let (m_id, mut e, r) = {
        let w = who.write().await;
        let r = from.read().await;
        log::trace!("Transport request by {} from {} to {}",
            w.id(), r.id(),
            to.read().await.id()
        );
        (w.tick_id(), w, r)
    };
    // see if the entity can open a lock, if `via` is locked.
    if let Some(exit) = r.exits.get(&via) {
        match exit {
            Exit::Locked { key_bp,.. }   |
            Exit::LockedAL { key_bp,.. } => {
                let Some(_) = e.inventory().find_id_by_name(key_bp) else { return /* no key, no go */;};
            }
            _ => ()
        }
    } else {
        log::error!("Where did the exit at '{}' from '{}' go!?", via, r.id());
        return ;
    }

    drop(e); drop(r);
    translocate!(ent who, m_id, from, to);
}

#[cfg(test)]
mod life_tests {
    use std::{io::Cursor, sync::Arc};

    use crate::{cmd::look::LookCommand, combat::{Battler, CombatantMut, DamageType}, r#const::SMALL_ITEM, get_operational_mock_janitor, get_operational_mock_librarian, identity::IdentityQuery, item::{Item, container::storage::Storage, ownership::Owner, weapon::{DEFAULT_WEAPON_SPEED, WeaponSize, WeaponSpec}}, room::environ::WEATHER_RAIN, stabilize_threads, thread::{SystemSignal, life::BattlerRec, signal::SpawnType}, util::access::Access, world::mock_world::get_operational_mock_world};

    #[cfg(all(feature = "obsolete", feature = "stresstest"))]
    #[tokio::test]
    async fn goblin_1_1_ocean() {
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
        let start_work = std::time::Instant::now();
        let (otx,orx) = tokio::sync::oneshot::channel::<()>();
        c.life.send(SystemSignal::CountSpawns { num: MILLION_GOBBOS, out: otx }).ok();
        for _ in 1..=MILLION_GOBBOS {
            c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        }
        // let the dust settle…
        let _ = orx.await;
        stabilize_threads!(10_000); // see for 10s what spams...
        let work_duration = start_work.elapsed();
        let spawns_per_sec = MILLION_GOBBOS as f64 / work_duration.as_secs_f64();
        let r1 = w.read().await.get_room_by_id("r-1").unwrap();
        let spawn_c = r1.read().await.entities.len();

        log::debug!("--terminated--");
        log::debug!("Duration: {work_duration:?} | Throughput: {spawns_per_sec:.2} ent/sec | Entities: {spawn_c}");
    }

    /// Goblin ocean
    /// 
    /// # Env vars
    /// - `GOBBOCOUNT` (non-stresstest) to define count of gobbos.
    /// - `GARDEN_TEST_RUNTIME` for runtime in millis within \[1_000, 30_000\].
    #[tokio::test]
    async fn goblin_ocean_batch() {
        #[cfg(feature = "stresstest")]
        let gobbocount: usize = 1_000_000;
        #[cfg(not(feature = "stresstest"))]
        let gobbocount: usize = std::env::var("GOBBOCOUNT")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<usize>()
            .unwrap_or_else(|_| 1_000);// chuck...
        let runtime: u64 = std::env::var("GARDEN_TEST_RUNTIME")
            .unwrap_or_else(|_| "5000".to_string())
            .parse::<u64>()
            .unwrap_or_else(|_| 5_000)// 5s default
            .clamp(1_000, 30_000);// clamp between 1s and 30s

        let (w,c,_,j) = get_operational_mock_world().await;
        get_operational_mock_janitor!(c,w,j.0);
        get_operational_mock_life!(c,w);
        get_operational_mock_librarian!(c,w);
        start_mock_broadcast_listener!(c);
        let c = c.out;// we don't need the c.recv part anymore here…
        stabilize_threads!();
        let start_work = std::time::Instant::now();
        let (otx,orx) = tokio::sync::oneshot::channel::<bool>();
        c.life.send(SystemSignal::SpawnBatch { what: SpawnType::Mob { id: "goblin".into() }, num: gobbocount, room: "r-1".into(), reply: otx.into() }).ok();
        // let the dust settle…
        let _ = orx.await;
        log::debug!("Dust?");
        let work_duration = start_work.elapsed();
        
        let r1 = { w.read().await.get_room_by_id("r-1").unwrap().clone() };
        // maybe stir the hornets' nest...
        {
            log::debug!("Setting rain to fall…");
            r1.write().await.set_special_env_bitmask(WEATHER_RAIN).ok();
        }

        #[cfg(feature = "stresstest")]      stabilize_threads!(30_000); // see for 30s what log fox says
        #[cfg(not(feature = "stresstest"))] stabilize_threads!(runtime);

        let spawns_per_sec = gobbocount as f64 / work_duration.as_secs_f64();
        let spawn_c = r1.read().await.entity_count();

        log::debug!("--terminated--");
        log::debug!("Duration: {work_duration:?} | Throughput: {spawns_per_sec:.2} ent/sec | Entities: {spawn_c}");
    }

    /// See about loot!
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
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into()}, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(100);
        log::debug!("Stabilized…");
        let lock = w.read().await;
        if let Some(r1) = lock.get_room_by_id(&"r-1") {
            let r1 = r1.clone();
            drop(lock);
            log::debug!("Dropped world lock…");
            let lock = r1.read().await;
            log::debug!("Found lil gobbo…");
            if let Some(e) = lock.get_entity_by_id("goblin").await {
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
                    dmg_type: DamageType::Cut,
                    speed: DEFAULT_WEAPON_SPEED,
                };
                let mut lock = e.write().await;
                lock.inventory().try_insert(Item::Weapon(spec)).ok();
                log::debug!("Gobbo has a stabber nao!");
                let erec = BattlerRec {
                    combatant: e.clone() as Battler,
                    title: Arc::from(lock.title().to_string()),
                };
                drop(lock);
                state = ctx!(state, LookCommand, "", s,c,w);
                log::debug!("Lootage…?");
                erec.loot_pinata(&w).await;
                log::debug!("Got loots…!");
            } else {
                panic!("Where did the gobbo go?! It was right here!");
            }
        } else {
            panic!("Ok, where did the room vanish?");
        }
        // little pause before final `look`.
        stabilize_threads!(100);
        p.write().await.access = Access::Builder;
        p.write().await.config.show_id = true;
        let _ = ctx!(state, LookCommand, "", s,c,w);
        log::debug!("--terminated--");
    }
}
