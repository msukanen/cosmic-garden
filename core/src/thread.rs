//! Nexus of threading…
pub(crate) mod janitor;
    pub(crate) use janitor::{janitor, add_item_to_lnf};
pub(crate) mod librarian;
    pub(crate) use librarian::librarian;
pub(crate) mod life;
    pub(crate) use life::life;
pub(crate) mod signal;
    pub(crate) use signal::SystemSignal;
pub(crate) mod per_client;

#[cfg(test)]
pub async fn stabilize_threads(countdown_ms: u64) {
    use std::time::Duration;

    tokio::time::sleep(Duration::from_millis(countdown_ms)).await;
}
#[macro_export]
macro_rules! stabilize_threads {
    () => {
        crate::thread::stabilize_threads(750).await;
    };
    
    ($countdown:expr) => {
        crate::thread::stabilize_threads($countdown).await;
    };
}
