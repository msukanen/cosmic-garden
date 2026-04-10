//! Lets look around…

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, player::Player, player_or_bust, string::Describable, tell_user};

pub struct LookCommand;

#[async_trait]
impl Command for LookCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);

        let Some(room) = plr.read().await.location.upgrade() else {
            tell_user!(ctx.writer, "You see absolutely nothing in the void.\n");
            return
        };

        let (title, desc, who, exits) = {
            let lock = room.read().await;
            (
                lock.title().to_string(),
                lock.desc().to_string(),
                lock.who.clone(),
                lock.exits.clone()
            )
        };
        let mut output: String = format!("<c yellow>{}</c>\n\n", title);
        output.push_str(&desc);
        output.push_str("\n\n");
        let ppl_arcs = who.iter()
            .filter_map(|(_,w)| w.upgrade())
            .collect::<Vec<Arc<RwLock<Player>>>>();
        let (plr_id, show_self) = {
            let p = plr.read().await;
            (p.id().to_string(), p.config.show_self_in_room)
        };
        for p in ppl_arcs {
            let lock = p.read().await;
            if lock.id() == plr_id && !show_self {
                continue;
            }
            output.push_str(&format!("  <c blue>[<c cyan>{}</c>]</c>\n", lock.title()));
        }
        output.push_str("\n<c green>Exits: </c>");
        let exs = exits.iter().map(|(dir,_)| dir.to_string()).collect::<Vec<String>>().join(", ");
        tell_user!(ctx.writer, "{}{}\n\n", output, exs);
    }
}
