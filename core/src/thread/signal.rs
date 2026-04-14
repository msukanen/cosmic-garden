//! System signals.

use tokio::sync::mpsc;

/// Various system signals between threads.
#[derive(Debug, Clone)]
pub enum SystemSignal {
    /// Generic "we're shutting down, brace for impact".
    Shutdown,
    /// Player in logout queue.
    PlayerInLogout,
    /// Save the whales, now!
    SaveWorld,
    ///
    LostAndFound,
    /// sent by Librarian -> IO, save the library
    /// sent by IO -> Librarian, reindex your aliases
    ReindexLibrary,
    /// New library entry, from e.g. builders.
    NewLibraryEntry,
}

#[derive(Debug, Clone)]
pub(crate) struct SignalChannels {
    pub janitor_tx: mpsc::Sender<SystemSignal>,
    pub librarian_tx: mpsc::Sender<SystemSignal>,
    pub game_tx: mpsc::Sender<SystemSignal>,
}

#[cfg(test)]
impl Default for SignalChannels {
    fn default() -> Self {
        let (jtx,_) = mpsc::channel::<SystemSignal>(2);
        let (ltx,_) = mpsc::channel::<SystemSignal>(2);
        let (gtx,_) = mpsc::channel::<SystemSignal>(2);
        Self { janitor_tx: jtx, librarian_tx: ltx, game_tx: gtx }
    }
}
