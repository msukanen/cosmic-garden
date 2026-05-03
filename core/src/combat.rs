//! Combat (and other bats) rules and stuff.

use std::sync::{Arc, Weak};

use tokio::sync::RwLock;

use crate::{identity::IdentityQuery, item::container::variants::ContainerVariant, mob::{Stat, StatError, StatValue}, room::Room};

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
    fn inventory(&mut self) -> &mut ContainerVariant;
    fn alter_brain_freeze(&mut self, freeze: bool);
    fn location_mut(&mut self) -> &mut Weak<RwLock<Room>>;
}

#[cfg(test)]
mod combatant_tests {
    use std::io::Cursor;

    use crate::{cmd::{attack::AttackCommand, get::GetCommand, look::LookCommand, wield::WieldCommand}, ctx, get_operational_mock_janitor, get_operational_mock_librarian, get_operational_mock_life, identity::{IdentityMut, IdentityQuery}, io::{Broadcast, ClientState}, mob::core::Entity, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, world::world_tests::get_operational_mock_world};

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
        let (w,c,(state, _),d) = get_operational_mock_world().await;
        let jt = get_operational_mock_janitor!(c,w,d.0);
        let gt = get_operational_mock_life!(c,w);
        let lt = get_operational_mock_librarian!(c,w);
        let c = c.out;// we don't need the c.recv part anymore here…
        stabilize_threads!();
        c.librarian.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room: "r-1".into(), reply: None }).ok();
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(150);
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

        tokio::spawn({async move {
            let mut b: Vec<u8> = vec![];
            let mut s = Cursor::new(&mut b);
            log::debug!("1st LookCommand warming up...");
            let state = ctx!(state, LookCommand, "",s,c,w,|out:&str| out.contains("goblin is here"));
            let state = ctx!(state, GetCommand, "knife", s,c,w,|out:&str| out.contains("nab"));
            let state = ctx!(state, WieldCommand, "knife", s,c,w,|out:&str| out.contains("wield"));
            log::debug!("AttackCommand warming up...");
            let _ = ctx!(state, AttackCommand, "goblin",s,c,w);
            log::debug!("AttackCommand fired.");
            stabilize_threads!(250);
            c.shutdown().await;
            log::debug!("Shutdown initiated.");
            return;
        }});

        _ = d.1.await;
        lt.await.ok();
        jt.await.ok();
        gt.await.ok();
    }
}
