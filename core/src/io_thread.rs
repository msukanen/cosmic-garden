//! Disk I/O threading…

use crate::DATA_PATH;

pub(super) async fn io_thread() {
    log::trace!("Firing up; DATA_PATH = '{}'", *DATA_PATH);
}
