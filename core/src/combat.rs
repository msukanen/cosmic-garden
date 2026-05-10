//! Combat (and other bats) rules and stuff.

use crate::mob::StatValue;

pub mod combatant; pub use combatant::*;
pub mod dmg; pub use dmg::*;

/// Generic "battler" type.
pub type Battler = std::sync::Arc<tokio::sync::RwLock<dyn CombatantMut + Send + Sync>>;

pub trait Damager {
    /// Get (current) dmg per attack.
    fn dmg(&self) -> StatValue;
    fn dmg_type(&self) -> DamageType;
}

#[cfg(test)]
mod combatant_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{attack::AttackCommand, get::GetCommand, look::LookCommand, wield::WieldCommand}, ctx, get_operational_mock_janitor, get_operational_mock_librarian, get_operational_mock_life, identity::{IdentityMut, IdentityQuery}, io::{Broadcast, ClientState}, mob::core::Entity, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, translocate, world::world_tests::get_operational_mock_world};

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
        let c = c.out;// we don't need the c.recv part anymore here…
        stabilize_threads!();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(25);
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(25);
        let mut rx = c.broadcast.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(b) = rx.recv() => match b {
                        Broadcast::MessageInRoom2 { message_actor, message_other, .. } => {
                            log::debug!("\n  → {message_actor}\n  → {message_other}");
                        }
                        Broadcast::BattleMessage3 { message_atk, message_other, message_vct, ..} => {
                            log::debug!("  atk: \"{message_atk}\"");
                            log::debug!("  vct: \"{message_vct}\"");
                            log::debug!("other: \"{message_other}\"");
                        }
                        _ => {}
                    }
                }
            }
        });
        stabilize_threads!(50);
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
        stabilize_threads!();
        c.librarian.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room: "r-1".into(), reply: None }).ok();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        let mut rx = c.broadcast.subscribe();
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
