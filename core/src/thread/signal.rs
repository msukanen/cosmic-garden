//! System signals.

use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};

use crate::{io::Broadcast, player::Player, room::Room};

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
    WantTransportFromTo { who: Arc<RwLock<Player>>, from: Arc<RwLock<Room>>, to: Arc<RwLock<Room>> },
    AbortBattleNow { who: String },
}

#[derive(Debug, Clone)]
pub(crate) struct SignalChannels {
    pub janitor_tx: mpsc::UnboundedSender<SystemSignal>,
    pub librarian_tx: mpsc::UnboundedSender<SystemSignal>,
    pub game_tx: mpsc::UnboundedSender<SystemSignal>,
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct SignalReceiverChannels {
    pub janitor_rx: mpsc::UnboundedReceiver<SystemSignal>,
    pub librarian_rx: mpsc::UnboundedReceiver<SystemSignal>,
    pub game_rx: mpsc::UnboundedReceiver<SystemSignal>,
}

#[cfg(test)]
impl SignalChannels {
    pub fn default() -> (Self, SignalReceiverChannels) {
        let (jtx,jrx) = mpsc::unbounded_channel::<SystemSignal>();
        let (ltx,lrx) = mpsc::unbounded_channel::<SystemSignal>();
        let (gtx,grx) = mpsc::unbounded_channel::<SystemSignal>();
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
        self.game_tx.send(SystemSignal::Shutdown);
        self.librarian_tx.send(SystemSignal::Shutdown);
        self.janitor_tx.send(SystemSignal::Shutdown);
    }
}
