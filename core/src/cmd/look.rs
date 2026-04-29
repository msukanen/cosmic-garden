//! Lets look around…

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, player::Player, player_or_bust, string::Describable, tell_user, util::access::Accessor};

pub struct LookCommand;

#[async_trait]
impl Command for LookCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);

        let Some(room) = plr.read().await.location.upgrade() else {
            tell_user!(ctx.writer, "You see absolutely nothing in the void.\n");
            return
        };
        let (is_builder, show_id) = {
            let p = plr.read().await;
            (p.access.is_builder(), p.config.show_id)
        };

        let (title, desc, who, exits, content, entities) = {
            let lock = room.read().await;
            (
                lock.title().to_string(),
                lock.desc().to_string(),
                lock.who.clone(),
                lock.exits.clone(),
                lock.into_iter().map(|(id,item)|
                    (id.clone(), item.title().to_string())
                ).collect::<Vec<_>>(),
                {
                    let mut ents = vec![];
                    for (id, ent) in lock.entities.iter() {
                        if let Ok(ent) = ent.try_read() {
                            ents.push((id.clone(), ent.title().to_string()));
                        } else {
                            ents.push((id.clone(), "Something moving fast".into()));
                        }
                    }
                    ents
                },
            )
        };

        let quick = ctx.args == "q";

        // Room lore:
        let mut output: String = String::new();
        
        if !quick {
            output.push_str(&format!("<c yellow>{}</c>\n\n", title));
            output.push_str(&desc);
            output.push('\n');
        }
        // Content:
        for (id,title) in content {
            if is_builder && show_id {
                output.push_str(&format!(" <c red>//</c> <c white>{}</c> <c gray>[{}]</c>\n", title, id));
            } else {
                output.push_str(&format!("  - {}\n", title));
            }
        }
        output.push('\n');
        // Entities:
        for (id,title) in &entities {
            if is_builder && show_id {
                output.push_str(&format!("  - {title}<c gray>({id})</c> is here…\n"));
            } else {
                output.push_str(&format!("  - {title} is here…\n"));
            }
        }
        if !entities.is_empty() {
            output.push('\n');
        }
        // People:
        let ppl_arcs = who.iter()
            .filter_map(|(_,w)| w.upgrade())
            .collect::<Vec<Arc<RwLock<Player>>>>();
        let show_self = {
            let p = plr.read().await;
            p.config.show_self_in_room
        };
        for p in ppl_arcs {
            let is_self = Arc::ptr_eq(&plr, &p);
            if is_self && !show_self {
                continue;
            } else if is_self {
                output.push_str("  <c blue>[<c cyan>***</c>]</c>\n");
            } else {
                output.push_str(&format!("  <c blue>[<c cyan>{}</c>]</c>\n", p.read().await.title()));
            }
        }
        let exs = if !quick {
            // Exits:
            output.push_str("\n<c green>Exits: </c>");
            exits.iter().map(|(dir,_)| dir.to_string()).collect::<Vec<String>>().join(", ")
        } else { "".into() };

        tell_user!(ctx.writer, "{}{}\n\n", output, exs);
    }
}
