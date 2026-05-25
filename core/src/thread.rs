//! Nexus of threading…
// the major players:
pub(crate) mod janitor;
pub(crate) use janitor::{janitor, add_item_to_lnf};

#[cfg(feature = "use-criterion")] pub mod librarian;
#[cfg(feature = "use-criterion")] pub use librarian::librarian;
#[cfg(not(feature = "use-criterion"))] pub(crate) mod librarian;
#[cfg(not(feature = "use-criterion"))] pub(crate) use librarian::librarian;

pub(crate) mod life;
pub(crate) use life::life;

pub(crate) mod per_client;

// signal system…
pub mod signal;
    pub use signal::SystemSignal;

/// Thread stabilizer. Awaits given `countdown_ms` (up to `60_000`ms).
// Exists mainly to ensure unsigned time value.
#[cfg(test)]
#[inline(always)]
pub async fn stabilize_threads(countdown_ms: u64) {
    use std::time::Duration;
    tokio::time::sleep(Duration::from_millis(countdown_ms.min(60_000))).await;
}

/// Thread stabilizer. Awaits given `countdown` (in millis) or default `750`ms.
#[cfg(test)]
#[macro_export]
macro_rules! stabilize_threads {
    () => {
        crate::thread::stabilize_threads(750).await;
    };
    
    ($countdown:expr) => {
        crate::thread::stabilize_threads($countdown).await;
    };
}
