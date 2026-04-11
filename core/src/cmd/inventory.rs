//! What's in the pockets?

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, player_or_bust, tell_user};

pub struct InventoryCommand;

#[async_trait]
impl Command for InventoryCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let (inv,show_id) = {
            let p = plr.read().await;
            (p.inventory.into_iter()
                .map(|(id,item)| (id.clone(), item.title().to_string()))
                .collect::<Vec<_>>(),
            p.config.show_id)
        };
        if inv.is_empty() {
            tell_user!(ctx.writer, "Your pockets are empty. Just lint and dreams.\n");
            return;
        }
        let mut output = String::from("You are carrying…\n");
        for (id, title) in inv {
            output.push_str(&{
                if show_id {format!(" <c red>//</c> {title} <c gray>{id}</c>\n")}
                else {format!("  - {title}\n")}
            });
        }
        tell_user!(ctx.writer, "{}", output);
    }
}
