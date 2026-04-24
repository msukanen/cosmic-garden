//! Combat (and other bats) rules and stuff.

use std::sync::{Arc, Weak};

use tokio::sync::RwLock;

use crate::{identity::IdentityQuery, mob::{Stat, StatError, StatValue}, room::Room};

/// Generic "battler" type.
pub type Battler = Arc<RwLock<dyn CombatantMut + Send + Sync>>;

pub trait Damager {
    /// Get (current) dmg per attack.
    fn dmg(&self) -> StatValue;
}

/// A trait for all "combatants".
pub trait Combatant: IdentityQuery + Damager {
    /// Get current HP (health points)
    fn hp<'a>(&'a self) -> &'a Stat;
    /// Get current MP (mental points)
    fn mp<'a>(&'a self) -> &'a Stat;
    /// Get current SN (strain points)
    fn sn<'a>(&'a self) -> &'a Stat;
    /// Get current SAN (sanity points)
    fn san<'a>(&'a self) -> &'a Stat;

    /// Get current Str(ength)
    fn str<'a>(&'a self) -> &'a Stat;
    /// Get current Nim(bleness)
    fn nim<'a>(&'a self) -> &'a Stat;
    /// Get current Brn(iness)
    fn brn<'a>(&'a self) -> &'a Stat;

    /// Is the combatant unconscious?
    fn is_unconscious(&self) -> Result<bool, StatError> {
        match (
            self.hp().is_unconscious(),
            self.mp().is_unconscious(),
            self.sn().is_unconscious(),
            self.san().is_unconscious(),
        ) {
            (Ok(true),..)    |
            (_,Ok(true),..)  |
            (_,_,Ok(true),..)|
            (_,_,_,Ok(true)) => Ok(true),
            _ => Ok(false)
        }
    }

    /// Is the [Combatant] dead?
    fn is_dead(&self) -> bool { self.hp().is_dead().ok().unwrap() }

    /// Get location of the [Combatant].
    fn location(&self) -> Weak<RwLock<Room>>;
}

/// Mutable trait for all "combatants".
pub trait CombatantMut : Combatant {
    /// Take damage.
    /// 
    /// # Args
    /// - `dmg` (usually to [HP][crate::mob::StatType::HP]).
    /// 
    /// # Return
    /// Was it a fatal blow?
    fn take_dmg(&mut self, dmg: StatValue) -> bool;
    /// Heal.
    /// 
    /// # Args
    /// - `dmg` (usually for [HP][crate::mob::StatType::HP]).
    // Technically more or less the reverse of [take_dmg()].
    fn heal(&mut self, dmg: StatValue);
    /// Get mutable HP.
    fn hp_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable MP.
    fn mp_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable SN.
    fn sn_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable San.
    fn san_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable Str.
    fn str_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable Brn.
    fn brn_mut<'a>(&'a mut self) -> &'a mut Stat;
    /// Get mutable Nim.
    fn nim_mut<'a>(&'a mut self) -> &'a mut Stat;
}

#[cfg(test)]
mod combatant_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{attack::AttackCommand, get::GetCommand, look::LookCommand, wield::WieldCommand}, ctx, get_operational_mock_janitor, get_operational_mock_librarian, get_operational_mock_life, identity::{IdentityMut, IdentityQuery}, io::{Broadcast, ClientState}, mob::core::Entity, tell_user, thread::{SystemSignal, librarian::librarian, life::life, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    /// Simulate 100 players' "gank squad" vs 1 (tough) goblin.
    /// 
    /// Estimated runtime in debug mode exactly 4.05s (including all the sleeps).
    #[tokio::test]
    async fn simple_combat() {
        let (w, mut c, p, _) = get_operational_mock_world().await;
        // let's accommodate the 100+ "players"…
        (c.out.broadcast, _) = tokio::sync::broadcast::channel::<Broadcast>( 128 );
        get_operational_mock_librarian!(c,w);
        get_operational_mock_life!(c,w);

        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…

        // Spawn a lil gobbo.
        let Ok(_) = Entity::new("goblin").await else { panic!("Where'd the lil goblin's blueprint go?!"); };
        let _ = c.out.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into() });
      
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
        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…
        {
            let c = c.out.clone();
            let w = w.clone();
            tokio::spawn(async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                let state = ClientState::Playing { player: p.clone() };
                let state = ctx!(state, LookCommand, "",s,c,w,p,|out:&str| out.contains("goblin is here"));
                let state = ctx!(state, AttackCommand, "goblin",s,c,w,p);
                tokio::time::sleep(Duration::from_secs(2)).await;
                let _ = ctx!(state, LookCommand, "",s,c,w,p,|out:&str| out.contains("goblin is here"));
            });
        }
        for x in 2..=100 {
        {
            let mut p2 = crate::player::Player::default();
            *(p2.id_mut()) = format!("test-player-{x}");
            let p2_id = p2.id().to_string();
            let p2 = std::sync::Arc::new(tokio::sync::RwLock::new(p2));
            w.write().await.players_by_id.insert(p2_id.clone(), p2.clone());
            let Some(r) = w.read().await.rooms.get("r-1").cloned() else { panic!("r-1 missing?!")};
            r.write().await.who.insert(p2_id.clone(), std::sync::Arc::downgrade(&p2));
            p2.write().await.location = std::sync::Arc::downgrade(&r);
            let c = c.out.clone();
            let w = w.clone();
            tokio::spawn(async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                let state = ClientState::Playing { player: p2.clone() };
                //let state = ctx!(state, LookCommand, "",s,c,w,p2,|out:&str| out.contains("goblin is here"));
                let _ = ctx!(state, AttackCommand, "goblin",s,c,w,p2);
            });
        }}
        tokio::time::sleep(Duration::from_secs(2)).await;
        log::debug!("--terminated--")
    }

    #[tokio::test(flavor="multi_thread")]
    async fn knife_fite() {
        let (w,c,p,d) = get_operational_mock_world().await;
        let jt = get_operational_mock_janitor!(c,w,d.0);
        let lt = get_operational_mock_librarian!(c,w);
        let gt = get_operational_mock_life!(c,w);
        let c = c.out;// we don't need the c.recv part anymore here…
        tokio::time::sleep(Duration::from_secs(2)).await;// let the threads stabilize…
        c.life.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room_id: "r-1".into() }).ok();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into()}).ok();
        tokio::time::sleep(Duration::from_secs(1)).await;// let the spawns stabilize…, Just in Case™
        let mut rx = c.broadcast.subscribe();
        let bcast = tokio::spawn(async move {
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
        log::debug!("still alive?");
        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…
        {
            tokio::spawn({async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                let state = ClientState::Playing { player: p.clone() };
                let state = ctx!(state, LookCommand, "",s,c,w,p,|out:&str| out.contains("goblin is here"));
                let state = ctx!(state, GetCommand, "knife", s,c,w,p,|out:&str| out.contains("nab"));
                let state = ctx!(state, WieldCommand, "knife", s,c,w,p,|out:&str| out.contains("wield"));
                let state = ctx!(state, AttackCommand, "goblin",s,c,w,p);
                log::debug!("AttackCommand fired. Waiting 6 seconds (or less) of combat to pass…");
                tokio::time::sleep(Duration::from_secs(6)).await;
                let _ = ctx!(state, LookCommand, "",s,c,w,p,|out:&str| out.contains("goblin is here"));
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
        let (w,c,p,d) = get_operational_mock_world().await;
        let jt = get_operational_mock_janitor!(c,w,d.0);
        let gt = get_operational_mock_life!(c,w);
        let lt = get_operational_mock_librarian!(c,w);
        let c = c.out;// we don't need the c.recv part anymore here…
        tokio::time::sleep(Duration::from_secs(2)).await;// let the threads stabilize…
        c.librarian.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room_id: "r-1".into() }).ok();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into()}).ok();
        tokio::time::sleep(Duration::from_secs(1)).await;// let the spawns stabilize…, Just in Case™
        //tokio::time::sleep(Duration::from_secs(2)).await;// let the threads stabilize…
        let mut rx = c.broadcast.subscribe();
        let bcast = tokio::spawn(async move {
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
        log::debug!("still alive?");
        tokio::time::sleep(Duration::from_secs(2)).await;// let things stabilize in peace…
        let senders = c.clone();

        tokio::spawn({async move {
            let mut b: Vec<u8> = vec![];
            let mut s = Cursor::new(&mut b);
            let state = ClientState::Playing { player: p.clone() };
            log::debug!("1st LookCommand warming up...");
            tokio::time::sleep(Duration::from_secs(2)).await;
            let state = ctx!(state, LookCommand, "",s,c,w,p,|out:&str| out.contains("goblin is here"));
            let state = ctx!(state, GetCommand, "knife", s,c,w,p,|out:&str| out.contains("nab"));
            let state = ctx!(state, WieldCommand, "knife", s,c,w,p,|out:&str| out.contains("wield"));
            log::debug!("AttackCommand warming up...");
            let _ = ctx!(state, AttackCommand, "goblin",s,c,w,p);
            log::debug!("AttackCommand fired.");
            tokio::time::sleep(Duration::from_millis(50)).await;
            senders.shutdown().await;
            log::debug!("Shutdown initiated.");
            return;
        }});

        _ = d.1.await;
        lt.await.ok();
        jt.await.ok();
        gt.await.ok();
    }
}
