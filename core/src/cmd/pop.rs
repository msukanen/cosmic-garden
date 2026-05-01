//! "Pop" something…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, combat::CombatantMut, identity::IdentityQuery, player_or_bust, roomloc_or_bust, tell_user, translocate, util::direction::Direction};

pub struct PopCommand;

#[async_trait]
impl Command for PopCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let origin = roomloc_or_bust!(plr);
        if ctx.args.is_empty() {
            tell_user!(ctx.writer, "Pop what? A balloon?\n");
            return ;
        }
        // don't pop balloons when in a balloon…
        match ctx.args.to_lowercase().as_str() {
            "balloon" => {
                let p_lock = plr.read().await;
                if let Some((dir, dest)) = p_lock.last_goto.clone() {
                    if dir == "balloon" {
                        if let Some(dest) = dest.upgrade() {
                            let p_id = p_lock.id().to_string();
                            drop(p_lock);
                            tell_user!(ctx.writer, "\"Pop!\" it goes, and you seem to be falling…\n");
                            translocate!(plr, p_id, origin, dest);
                            {
                                let mut p_lock = plr.write().await;
                                p_lock.last_goto = None;
                                p_lock.take_dmg(5.0);
                            }
                            let mut d = dest.write().await;
                            d.exits.retain(|dir,_| if let Direction::Custom(d) = dir {
                                d != "balloon"
                            } else {true});
                            return ;
                        }
                    }
                }
            }
            _ => ()
        }

        // TODO: actually pop something…
        tell_user!(ctx.writer, "Nah, it's not poppable… Or maybe is, but you decide not to try really.\n");
    }
}
