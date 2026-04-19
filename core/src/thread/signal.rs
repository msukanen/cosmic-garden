//! System signals.

use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};

use crate::{io::Broadcast, player::Player};

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

    ///--- Life Thread
    /// Notion to spawn something…
    Spawn { what: SpawnType, room_id: String },
    Attack { who: Arc<RwLock<Player>>, victim_id: String },
    PlayerLogout { who: String },
    PlayerLogin { who: String, tx: tokio::sync::broadcast::Sender<Broadcast> },
}

#[derive(Debug, Clone)]
pub(crate) struct SignalChannels {
    pub janitor_tx: mpsc::Sender<SystemSignal>,
    pub librarian_tx: mpsc::Sender<SystemSignal>,
    pub game_tx: mpsc::Sender<SystemSignal>,
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct SignalReceiverChannels {
    pub janitor_rx: mpsc::Receiver<SystemSignal>,
    pub librarian_rx: mpsc::Receiver<SystemSignal>,
    pub game_rx: mpsc::Receiver<SystemSignal>,
}

#[cfg(test)]
impl SignalChannels {
    pub fn default() -> (Self, SignalReceiverChannels) {
        let (jtx,jrx) = mpsc::channel::<SystemSignal>(2);
        let (ltx,lrx) = mpsc::channel::<SystemSignal>(2);
        let (gtx,grx) = mpsc::channel::<SystemSignal>(2);
        (   Self {
                janitor_tx: jtx,
                librarian_tx: ltx,
                game_tx: gtx
            },
            SignalReceiverChannels {
                janitor_rx: jrx,
                librarian_rx: lrx,
                game_rx: grx
            }
        )
    }
}

/// Spawn types for life-thread signals.
#[derive(Debug, Clone)]
pub enum SpawnType {
    Mob { id: String },
    Item { id: String },
}

impl SignalChannels {
    pub async fn shutdown(&self) {
        self.game_tx.send(SystemSignal::Shutdown).await;
        self.librarian_tx.send(SystemSignal::Shutdown).await;
        self.janitor_tx.send(SystemSignal::Shutdown).await;
    }
}
