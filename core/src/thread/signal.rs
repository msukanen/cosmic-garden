//! System signals.

use std::{sync::Arc, time::Duration};

use tokio::sync::{RwLock, broadcast, mpsc};

use crate::{combat::Battler, io::Broadcast, player::Player, room::Room, thread::life::TickType, util::direction::Direction};

pub type SigReceiver = mpsc::UnboundedReceiver<SystemSignal>;
pub type SigSender = mpsc::UnboundedSender<SystemSignal>;

/// Various system signals between threads.
pub enum SystemSignal {
    /// Generic "we're shutting down, brace for impact".
    Shutdown,

    //--- Janitor ---
    /// Player logging out, queue or otherwise.
    PlayerNeedsSaving (Arc<RwLock<Player>>),
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
    Attack { atk_arc: Battler, vct_arc: Battler },
    PlayerLogout { player: Arc<RwLock<Player>> },
    WantTransportFromTo { who: Arc<RwLock<Player>>, from: Arc<RwLock<Room>>, to: Arc<RwLock<Room>>, via: Direction },
    AbortBattleNow { who: Battler },
    /// Query life-thread how many ticks is `sec`.
    SecToTicks { sec: u32, tick_type: TickType, out: tokio::sync::oneshot::Sender<u32> },
    #[cfg(test)]
    CountSpawns { num: usize, out: tokio::sync::oneshot::Sender<()> },
    AlterTickRate { tick_type: TickType, duration: Duration },
}

#[derive(Debug, Clone)]
pub struct SignalSenderChannels {
    pub broadcast: broadcast::Sender<Broadcast>,
    pub janitor: SigSender,
    pub librarian: SigSender,
    pub life: SigSender,
}

#[derive(Debug)]
pub(crate) struct SignalReceiverChannels {
    pub janitor: SigReceiver,
    pub librarian: SigReceiver,
    pub life: SigReceiver,
}

pub(crate) struct SignalChannels {
    pub out: SignalSenderChannels,
    pub recv: SignalReceiverChannels,
}

impl SignalChannels {
    pub fn default() -> Self {
        let (tx, _) = broadcast::channel::<Broadcast>(16);
        let (jtx,jrx) = mpsc::unbounded_channel::<SystemSignal>();
        let (ltx,lrx) = mpsc::unbounded_channel::<SystemSignal>();
        let (gtx,grx) = mpsc::unbounded_channel::<SystemSignal>();
        Self {
            out: SignalSenderChannels {
                broadcast: tx,
                janitor: jtx,
                librarian: ltx,
                life: gtx,
            },
            recv: SignalReceiverChannels {
                janitor: jrx,
                librarian: lrx,
                life: grx,
            }
        }
    }
}

/// Spawn types for life-thread signals.
#[derive(Debug, Clone)]
pub enum SpawnType {
    Mob { id: String },
    Item { id: String },
}

impl SignalSenderChannels {
    pub async fn shutdown(&self) {
        self.life.send(SystemSignal::Shutdown).ok();
        self.librarian.send(SystemSignal::Shutdown).ok();
        self.janitor.send(SystemSignal::Shutdown).ok();
    }
}
