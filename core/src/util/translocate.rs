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
    ($plr:ident, $origin:expr, $target:expr) => {
        {
            let p_id = $plr.read().await.id().to_string();
            $origin.write().await.who.remove(&p_id);
            $target.write().await.who.insert(p_id.clone(), Arc::downgrade(&$plr));
            $plr.write().await.location = Arc::downgrade(&$target);
        }
    };

    ($plr:ident, $p_id:ident, $origin:expr, $target:expr) => {
        {
            $origin.write().await.who.remove(&$p_id);
            $target.write().await.who.insert($p_id.clone(), Arc::downgrade(&$plr));
            $plr.write().await.location = Arc::downgrade(&$target);
        }
    };
}
