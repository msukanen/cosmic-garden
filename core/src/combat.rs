//! Combat (and other bats) rules and stuff.

use crate::{identity::IdentityQuery, item::weapon::WeaponSize, mob::StatValue};

/// A trait for all "combatants".
pub trait Combatant: IdentityQuery {
    /// Get (current) dmg per attack.
    fn dmg(&self) -> StatValue;
    /// Get maximum weapon size the combatant can wield.
    fn max_weapon_size(&self) -> WeaponSize;
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
}

#[cfg(test)]
mod combatant_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{attack::AttackCommand, look::LookCommand}, ctx, identity::{IdentityMut, IdentityQuery}, io::ClientState, mob::core::Entity, thread::{SystemSignal, librarian::librarian, life_thread::life_thread, signal::SpawnType}, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn simple_combat() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w, tx, c, p) = get_operational_mock_world().await;
        tokio::spawn(librarian((c.0.clone(), c.1.librarian_rx)));
        tokio::spawn(life_thread((c.0.clone(), c.1.game_rx), w.clone()));
        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…

        // Spawn a lil gobbo.
        let Ok(_) = Entity::new("goblin").await else { panic!("Where'd the lil goblin's blueprint go?!"); };
        let _ = c.0.game_tx.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into() });

        // Player 2.
        let mut p2 = crate::player::Player::default();
        *(p2.id_mut()) = "test-player-2".to_string();
        let p2_id = p2.id().to_string();
        let p2 = std::sync::Arc::new(tokio::sync::RwLock::new(p2));
        w.write().await.players_by_id.insert(p2_id.clone(), p2.clone());
        let Some(r) = w.read().await.rooms.get("r-1").cloned() else { panic!("r-1 missing?!")};
        r.write().await.who.insert(p2_id.clone(), std::sync::Arc::downgrade(&p2));
        p2.write().await.location = std::sync::Arc::downgrade(&r);

        tokio::time::sleep(Duration::from_secs(1)).await;// let things stabilize in peace…
        
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, LookCommand, "",s,tx,c,w,p,|out:&str| out.contains("goblin is here"));
        let state = ctx!(state, AttackCommand, "goblin",s,tx,c,w,p);
        
        let state2 = ClientState::Playing { player: p2.clone() };
        let state2 = ctx!(state2, AttackCommand, "goblin",s,tx,c,w,p);
        tokio::time::sleep(Duration::from_secs(60)).await;
        log::debug!("--terminated--")
    }
}
