//! "Pop" something…

use async_trait::async_trait;

pub struct PopCommand;

#[async_trait]
impl Command