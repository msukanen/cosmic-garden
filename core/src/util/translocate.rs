//! Hauling things.
/// Translocate player to another place.
/// 
/// # Args
/// - `$plr`— [Player] arc<rwlock>
/// - `$p_id`— [Player].id, if fetched already before.
/// - `$origin`— [Room] arc<rwlock>
/// - `$target`— [Room] arc<rwlock>
/// 
/// If `translocate!(a,b,c)` deadlocks, get ID earlier and use `translocate!(a,b,c,d)`.
#[macro_export]
macro_rules! translocate {
    // Translocate [$plr][Player] from [$origin][Room] to [$target][Room]
    ($plr:expr, $origin:expr, $target:expr) => {
        {
            use crate::identity::IdentityQuery;
            let p_id = $plr.read().await.id().to_string();
            $origin.write().await.who.remove(&p_id);
            $target.write().await.who.insert(p_id.clone(), std::sync::Arc::downgrade(&$plr));
            $plr.write().await.set_location(&$target).await;
        }
    };

    // Translocate [$plr][Player] (ID $p_id) from [$origin][Room] to [$target][Room]
    ($plr:expr, $p_id:ident, $origin:expr, $target:expr) => {
        {
            $origin.write().await.who.remove(&$p_id);
            $target.write().await.who.insert($p_id.clone(), std::sync::Arc::downgrade(&$plr));
            $plr.write().await.set_location(&$target).await;
        }
    };

    // Translocate [$ent][Entity] (ID $id) from [$origin][Room] to [$target][Room]
    (ent $ent:expr, $id:ident, $origin:expr, $target:expr) => {
        {
            use crate::combat::CombatantMut;
            $origin.write().await.entities.remove(&$id);
            $target.write().await.entities.insert($id.clone(), $ent.clone());
            *($ent.write().await.location_mut()) = std::sync::Arc::downgrade(&$target);
        }
    };
}
