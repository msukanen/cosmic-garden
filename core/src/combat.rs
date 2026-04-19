//! Combat (and other bats) rules and stuff.

use crate::mob::StatValue;

pub trait Combatant {
    fn dmg(&self) -> StatValue;
}

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
}

#[cfg(test)]
mod combatant_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{attack::AttackCommand, look::LookCommand}, ctx, io::ClientState, mob::core::Entity, thread::{SystemSignal, librarian::librarian, life_thread::life_thread, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn simple_combat() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w, tx, mut c, p) = get_operational_mock_world().await;
        tokio::spawn(librarian((c.0.clone(), c.1.librarian_rx)));
        tokio::spawn(life_thread((c.0.clone(), c.1.game_rx), w.clone()));
        tokio::time::sleep(Duration::from_secs(2)).await;// let things stabilize in peace…
        let Ok(mob) = Entity::new("goblin").await else {
            panic!("Where'd the lil goblin go?!");
        };
        let _ = c.0.game_tx.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into() }).await;
        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, LookCommand, "",s,tx,c,w,p,|out:&str| out.contains("goblin is here"));
        let state = ctx!(state, AttackCommand, "goblin",s,tx,c,w,p);
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
