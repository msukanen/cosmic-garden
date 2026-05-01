//! Who's online?

use std::sync::Arc;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, player_or_bust, string::styling::RULER_LINE_PLAIN, util::access::Accessor, tell_user};

pub struct WhoCommand;

#[async_trait]
impl Command for WhoCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let is_staff = plr.read().await.access.is_true_builder();
        let mut output = String::from("<c green>[ Players in the Cosmic Garden ]</c>\n");
        output.push_str(&format!("{}\n", RULER_LINE_PLAIN));

        if is_staff {
            output.push_str(&format!("    <c yellow>{:<14}</c> | {:<21} | {:<14} | State\n", "ID", "Name", "Location"));
            output.push_str(&format!("{}\n", RULER_LINE_PLAIN));
        }

        let mut seen = 0;
        let world = ctx.world.read().await;
        for (id, p_arc) in &world.players_by_id {
            let p = p_arc.read().await;
            if p.config.is_ghost && !is_staff { continue; }
            let title = if Arc::ptr_eq(&plr, &p_arc) { "<you>" } else { p.title() };
            let loc_id = p.location_id.clone();
            
            if is_staff {
                output.push_str(&format!("<c cyan>{:<18}</c> | {:<21} | {:<14} | {}\n", id, title, loc_id, p.activity_type));
            } else {
                output.push_str(&format!(" <c blue>*</c> {}\n", title));
            }
            seen += 1;
        }
        
        output.push_str(&format!("{}\nTotal Souls: {}\n",
            RULER_LINE_PLAIN,
            if is_staff { world.players_by_id.len() } else { seen }
        ));
        tell_user!(ctx.writer, "{}", output);
    }
}
