//! Disk I/O threading…

use std::{collections::VecDeque, sync::Arc};

use lazy_static::lazy_static;
use tokio::sync::RwLock;

use crate::{DATA_PATH, player::Player};

lazy_static! {
    pub static ref PLAYERS_TO_LOGOUT: Arc<RwLock<VecDeque<Arc<RwLock<Player>>>>> = Arc::new(RwLock::new(VecDeque::new()));
}

/// Disk I/O thread thing.
pub(super) async fn io_thread() {
    log::trace!("Firing up; DATA_PATH = '{}'", *DATA_PATH);
}
