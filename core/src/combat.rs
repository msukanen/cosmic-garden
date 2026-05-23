//! Combat (and other bats) rules and stuff.

use std::sync::Arc;

use crate::{item::Item, lock2key, mob::StatValue, room::RoomArc, string::DescribableMut, thread::{add_item_to_lnf, life::BattleStage}, traits::Reflector, world::WorldArc};

pub mod combatant; pub use combatant::*;
pub mod dmg; pub use dmg::*;

/// Generic "battler" type.
pub type Battler = std::sync::Arc<tokio::sync::RwLock<dyn CombatantMut + Send + Sync>>;

pub trait Damager {
    /// Get (current) dmg per attack.
    fn dmg(&self, battle_tick: usize) -> Option<StatValue>;
    fn dmg_type(&self) -> DamageType;
}

#[derive(Clone)]
pub(super) struct BattlerRec {
    pub combatant: Battler,
    pub title: std::sync::Arc<str>,
}

impl BattlerRec {
    pub(super) async fn loot_pinata(&self, world: &WorldArc) {
        let mut lock = self.combatant.write().await;
        if let Some(room) = lock.location().upgrade() {
            let c_id = lock.tick_id();
            if !world.read().await.entities.contains_key(&c_id) {
                // alerady looted, bail.
                return ;
            }
            let mut c_inv = Item::Corpse { loot: lock.inventory().deep_reflect(), size: 50 };
            let c_title = lock.title().to_string();
            drop(lock);
            c_inv.set_desc(&format!("Corpse of '{}'", c_title));
            {
                let mut lock = room.write().await;
                lock.remove_entity(c_id);
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

/// Combat resolutions.
#[derive(Debug, Clone)]
pub enum Resolution {
    Inconclusive { atk_dmg: Option<StatValue>, vct_dmg: Option<StatValue> },
    AtkRetreat,
    VctRetreat,
    AtkVictory { atk_dmg: Option<StatValue> },
    VctVictory  { vct_dmg: Option<StatValue> },
    BothDead,
    AbortDueRealityWarp,
}

/// Fite!
/// 
/// # Args
/// - `battle_tick` as of right now for…
/// - `atk` vs.
/// - `vct`
/// - …at `room`.
pub(super) async fn punt(battle_tick: usize, atk: Battler, vct: Battler, room: &RoomArc) -> Resolution {
    let mut a = atk.write().await;
    let mut v = vct.write().await;
    // reality warp just before .writes?
    let wr = Arc::downgrade(room);
    if !a.location().ptr_eq(&wr) || !v.location().ptr_eq(&wr) {
        return Resolution::AbortDueRealityWarp;
    }

    let atk_dmg = a.dmg(battle_tick);
    let v_ded = v.take_dmg(atk_dmg);
    let (a_ded, vct_dmg) = if v_ded {
        // potential last-breath counter before falling over...
        //a.take_dmg(v.dmg());
        (false, 0.0.into())
    } else {
        let vct_dmg = v.dmg(battle_tick);
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

/// Register Ok'd battle.
pub(super) async fn register_ok_battle(atk: BattlerRec, vct: BattlerRec, room: RoomArc, bs: &mut BattleStage) {
    let a_key = lock2key!(arc &atk.combatant);
    let v_key = lock2key!(arc &vct.combatant);
    {
        let mut a_lock = atk.combatant.write().await;
        let mut v_lock = vct.combatant.write().await;
        a_lock.alter_brain_freeze(true);
        v_lock.alter_brain_freeze(true);
    }
    bs.active.insert(a_key, (atk, room.clone()));
    bs.active.insert(v_key, (vct, room.clone()));
    
    // A
    if let Some(a) = bs.atk.get_mut(&a_key) {
        if !a.contains(&v_key) {
            a.push(v_key);
        }
    } else {
        log::trace!("New attacker: {a_key}");
        bs.atk.insert(a_key, vec![v_key]);
    }

    // V
    if let Some(v) = bs.vct.get_mut(&v_key) {
        if !v.contains(&a_key) {
            v.push(a_key);
        }
    } else {
        log::trace!("New victim: {v_key}");
        bs.vct.insert(v_key, vec![a_key]);
    }
    log::trace!("LifeworkerSignal::BattleOk!");
}

#[cfg(test)]
mod combatant_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{attack::AttackCommand, get::GetCommand, look::LookCommand, wield::WieldCommand}, ctx, get_operational_mock_janitor, get_operational_mock_librarian, get_operational_mock_life, identity::{IdentityMut, IdentityQuery}, io::{Broadcast, ClientState}, mob::core::Entity, stabilize_threads, start_mock_broadcast_listener, thread::{SystemSignal, signal::SpawnType}, translocate, world::mock_world::get_operational_mock_world};

    /// Simulate 100 players' "gank squad" vs 1 (tough) goblin.
    /// 
    /// Estimated runtime in debug mode exactly 4.05s (including all the sleeps).
    #[tokio::test]
    async fn simple_combat() {
        let (w, mut c,(_, p),_) = get_operational_mock_world().await;
        // let's accommodate the 100+ "players"…
        (c.out.broadcast, _) = tokio::sync::broadcast::channel::<Broadcast>( 128 );
        get_operational_mock_librarian!(c,w);
        get_operational_mock_life!(c,w);

        stabilize_threads!();

        // Spawn a lil gobbo.
        let Ok(_) = Entity::new("goblin", &c.out).await else { panic!("Where'd the lil goblin's blueprint go?!"); };
        let _ = c.out.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None });
        stabilize_threads!(25);
      
        let mut rx = c.out.broadcast.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(b) = rx.recv() => match b {
                        Broadcast::MessageInRoom2 { message_actor, message_other, .. } => {
                            log::debug!("\n  → {message_actor}\n  → {message_other}");
                        },
                        _ => {}
                    }
                }
            }
        });
        {
            let c = c.out.clone();
            let w = w.clone();
            tokio::spawn(async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                let state = ClientState::Playing { player: p.clone() };
                let state = ctx!(state, LookCommand, "",s,c,w,|out:&str| out.contains("goblin is here"));
                let state = ctx!(state, AttackCommand, "goblin",s,c,w);
                stabilize_threads!();
                let _ = ctx!(state, LookCommand, "",s,c,w,|out:&str| out.contains("corpse"));
            });
        }
        for x in 2..=100 {
        {
            let mut p2 = crate::player::Player::default();
            p2.set_id(&format!("test-player-{x}"), true).ok();
            let p2_id = p2.id().to_string();
            let p2 = std::sync::Arc::new(tokio::sync::RwLock::new(p2));
            w.write().await.players_by_id.insert(p2_id.clone(), p2.clone());
            let Some(r) = w.read().await.get_room_by_id(&"r-1").clone() else { panic!("r-1 missing?!")};
            r.write().await.who.insert(p2_id.clone(), std::sync::Arc::downgrade(&p2));
            p2.write().await.location = std::sync::Arc::downgrade(&r);
            let c = c.out.clone();
            let w = w.clone();
            tokio::spawn(async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                let state = ClientState::Playing { player: p2.clone() };
                let _ = ctx!(state, AttackCommand, "goblin",s,c,w);
            });
        }}
        stabilize_threads!(2000);
        log::debug!("--terminated--")
    }

    #[tokio::test(flavor="multi_thread")]
    async fn knife_fite() {
        let (w,c,(_, p),d) = get_operational_mock_world().await;
        let jt = get_operational_mock_janitor!(c,w,d.0);
        let lt = get_operational_mock_librarian!(c,w);
        let gt = get_operational_mock_life!(c,w);
        let c = c.out;
        start_mock_broadcast_listener!(c);
        stabilize_threads!();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(25);
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(25);
        {
            tokio::spawn({async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                let state = ClientState::Playing { player: p.clone() };
                let state = ctx!(state, LookCommand, "",s,c,w,|out:&str| out.contains("goblin is here"));
                let state = ctx!(state, GetCommand, "knife", s,c,w,|out:&str| out.contains("nab"));
                let state = ctx!(state, WieldCommand, "knife", s,c,w,|out:&str| out.contains("wield"));
                let state = ctx!(state, AttackCommand, "goblin",s,c,w);
                static STAB_TIME: u64 = 5000;
                log::debug!("AttackCommand fired. Waiting {STAB_TIME}ms (or less) of combat to pass…");
                stabilize_threads!(STAB_TIME);
                let _ = ctx!(state, LookCommand, "",s,c,w,|out:&str| out.contains("corpse-inventory"));
                c.shutdown().await;
            }});
        }

        _ = d.1.await;
        lt.await.ok();
        jt.await.ok();
        gt.await.ok();
    }

    #[tokio::test]
    async fn player_vanish_midcombat() {
        let (w,c,(mut state,p),d) = get_operational_mock_world().await;
        let jt = get_operational_mock_janitor!(c,w,d.0);
        let gt = get_operational_mock_life!(c,w);
        let lt = get_operational_mock_librarian!(c,w);
        let c = c.out;// we don't need the c.recv part anymore here…
        start_mock_broadcast_listener!(c);
        stabilize_threads!();
        c.librarian.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room: "r-1".into(), reply: None }).ok();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();

        // Get combat rolling…
        tokio::spawn({
            let combat_w = w.clone();
            async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                log::debug!("1st LookCommand warming up...");
                stabilize_threads!(100);// give life a moment, Just in Case™
                state = ctx!(state, LookCommand, "",s,c,combat_w,|out:&str| out.contains("goblin is here"));
                state = ctx!(state, GetCommand, "knife", s,c,combat_w,|out:&str| out.contains("nab"));
                state = ctx!(state, WieldCommand, "knife", s,c,combat_w,|out:&str| out.contains("wield"));
                log::debug!("AttackCommand warming up...");
                _ = ctx!(state, AttackCommand, "goblin",s,c,combat_w);
                log::debug!("Combat max <2500ms, but expected to get interrupted much sooner.");
                stabilize_threads!(2500);
                c.shutdown().await;
            }
        });

        let r1 = w.read().await.get_room_by_id("r-1").clone().unwrap();
        let r2 = w.read().await.get_room_by_id("r-2").clone().unwrap();
        // Prep yanker…
        tokio::time::sleep(Duration::from_millis(250)).await;// 250ms should be enough of fite
        log::debug!("Yoink!");
        translocate!(p,r1,r2);
        log::debug!("Lets let dust settle…");
        tokio::time::sleep(Duration::from_millis(1000)).await;// 1000ms should be enough a wait

        _ = d.1.await;
        lt.await.ok();
        jt.await.ok();
        gt.await.ok();
    }
}
