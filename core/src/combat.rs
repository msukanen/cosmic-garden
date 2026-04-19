//! Combat (and other bats) rules and stuff.

use crate::mob::StatValue;

pub trait Combatant {

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

    use tokio::sync::mpsc;

    use crate::{thread::{SystemSignal, life_thread::life_thread}, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn simple_combat() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w, tx, mut ch, p) = get_operational_mock_world().await;
        let (gtx, grx) = mpsc::channel::<SystemSignal>(64);
        tokio::spawn( life_thread((ch.0, ch.1.game_rx), w.clone()) );
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
