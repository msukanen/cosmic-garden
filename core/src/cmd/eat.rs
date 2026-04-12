//! Eat something! You need it…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, io_thread::add_item_to_lnf, item::{Item, container::Storage, matter::MatterState}, mob::affect::{Affector, stack_affect}, player_or_bust, roomloc_or_bust, string::Uuid, tell_user};

pub struct EatCommand;

#[async_trait]
impl Command for EatCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let p_loc = roomloc_or_bust!(plr);

        // Let see if there's anything nommable…
        let Some(mut item) = plr.write().await.inventory.take_by_name(ctx.args) else {
            tell_user!(ctx.writer, "Even turning your pockets inside out, you can't find anything like '{}'…\n", ctx.args);
            return ;
        };
        let item_name = item.title().to_string();
        let Item::Consumable(ref mut m) = item else {
            let Err(e) = plr.write().await.receive_item(item) else {
                tell_user!(ctx.writer, "Ick! You can't possibly even think about consuming '{}'!\n", item_name);
                return;
            };
            let Err(e) = p_loc.write().await.contents.try_insert(e.extract_item().unwrap()) else {
                tell_user!(ctx.writer, "Whoops, greasy fingers, you dropped '{}'…!\n", item_name);
                return ;
            };
            add_item_to_lnf(e.extract_item().unwrap()).await;
            tell_user!(ctx.writer, "Sheesh, the nerve! Something or someone stole your '{}'?!\n", item_name);
            return;
        };

        let mut used_up = false;
        if let Some(ref mut uses) = m.uses {
            *uses -= 1;
            used_up = *uses == 0;
        }
        
        if let Some(affect) = m.as_affect() {
            tell_user!(ctx.writer, "You {} the '{}'. {}\n",
                m.matter_state.delivery_method(),
                item_name,
                match m.matter_state {
                    MatterState::Liquid => "*Glug-glug*",
                    MatterState::Solid => "Not bad…",
                    MatterState::Gaseous => "Which makes you hickup.",
                    MatterState::Plasma => "GAH! Probably <c red>bad idea</c>…"
                }
            );
            let mut p = plr.write().await;
            stack_affect(item.id(), &affect, &mut p.affects);
        }
        else {tell_user!(ctx.writer, "Bah! Tastes like cardboard…!\n");}

        if used_up {
            // Item out of uses, just drop the mic.
            return;
        }
        
        let Err(e) = plr.write().await.receive_item(item) else { return; };
        let Err(e) = p_loc.write().await.contents.try_insert(e.extract_item().unwrap()) else {
            tell_user!(ctx.writer, "Woops, … and you dropped it. Bah, double bah!\n");
            return;
        };
        add_item_to_lnf(e.extract_item().unwrap()).await;
        tell_user!(ctx.writer, "*mutter* where'd it go?\n");
    }
}
