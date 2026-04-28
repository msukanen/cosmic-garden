//! Clone things (not clowns though)!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, item::container::Storage, show_help_if_needed, traits::Reflector, validate_access};

pub struct CloneCommand;

#[async_trait]
impl Command for CloneCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, true_builder);
        show_help_if_needed!(ctx, "clone");

        if let Some(item) = plr.read().await.inventory.peek_at(ctx.args) {
            let new = item.deep_reflect();
            
        }
    }
}
