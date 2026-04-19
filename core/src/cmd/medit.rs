//! MEdit – a.k.a. "mob editor".

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, validate_access};

include!(concat!(env!("OUT_DIR"), "/medit_registry.rs"));

pub struct MeditCommand;

#[async_trait]
impl Command for MeditCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, true_builder);
        
    }
}