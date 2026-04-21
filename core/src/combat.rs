//! Combat (and other bats) rules and stuff.

use crate::{identity::IdentityQuery, mob::{Stat, StatError, StatValue}};

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

    use crate::{cmd::{attack::AttackCommand, look::LookCommand}, ctx, identity::{IdentityMut, IdentityQuery}, io::{Broadcast, ClientState}, mob::core::Entity, tell_user, thread::{SystemSignal, librarian::librarian, life::life, signal::SpawnType}, world::world_tests::get_operational_mock_world};

    /// Simulate 100 players' "gank squad" vs 1 (tough) goblin.
    /// 
    /// Estimated runtime in debug mode exactly 14.03s (including all the sleeps).
    #[tokio::test]
    async fn simple_combat() {
        let (w, mut c, p) = get_operational_mock_world().await;
        (c.out.broadcast, _) = tokio::sync::broadcast::channel::<Broadcast>( 128 );

        tokio::spawn(librarian((c.out.clone(), c.recv.librarian)));
        tokio::spawn(life((c.out.clone(), c.recv.life), w.clone()));

        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…

        c.out.life.send(SystemSignal::PlayerLogin { id: p.read().await.id().into(), title: p.read().await.title().into() }).ok();

        // Spawn a lil gobbo.
        let Ok(_) = Entity::new("goblin").await else { panic!("Where'd the lil goblin's blueprint go?!"); };
        let _ = c.out.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into() });
      
        let mut rx = c.out.broadcast.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(b) = rx.recv() => match b {
                        Broadcast::SystemInRoom { message_actor, message_other, .. } => {
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
                tokio::time::sleep(Duration::from_secs(8)).await;
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
            c.out.life.send(SystemSignal::PlayerLogin { id: p2.read().await.id().into(), title: p2.read().await.title().into() }).ok();
            let c = c.out.clone();
            let w = w.clone();
            tokio::spawn(async move {
                let mut b: Vec<u8> = vec![];
                let mut s = Cursor::new(&mut b);
                let state = ClientState::Playing { player: p2.clone() };
                //let state = ctx!(state, LookCommand, "",s,c,w,p2,|out:&str| out.contains("goblin is here"));
                let _ = ctx!(state, AttackCommand, "goblin",s,c,w,p2);
                tokio::time::sleep(Duration::from_secs(10)).await;
            });
        }}
        tokio::time::sleep(Duration::from_secs(12)).await;
        log::debug!("--terminated--")
    }
}
