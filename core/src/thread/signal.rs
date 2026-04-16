//! System signals.

use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};

use crate::player::Player;

/// Various system signals between threads.
#[derive(Debug, Clone)]
pub enum SystemSignal {
    /// Generic "we're shutting down, brace for impact".
    Shutdown,

    //--- Janitor ---
    /// Player logging out, queue or otherwise.
    PlayerNeedsSaving (Arc<RwLock<Player>>, String),
    /// Save the whales, now!
    SaveWorld,
    /// Item tucked into L'n'F.
    LostAndFound,

    //--- Librarian ---
    /// sent by Librarian -> IO, save the library
    /// sent by IO -> Librarian, reindex your aliases
    ReindexLibrary,
    /// New library entry, from e.g. builders.
    NewLibraryEntry,
    /// New blueprint entry, from e.g. builders.
    NewBlueprintEntry,
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
