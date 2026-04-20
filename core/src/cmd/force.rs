//! Use the Force, pronto!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, io::{Broadcast, ClientState, ForceTarget}, tell_user, validate_access};

pub struct ForceCommand;

#[async_trait]
impl Command for ForceCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, admin);

        let (admin_title, is_ghost, admin_loc) = {
            let p = plr.read().await;
            let p_id = p.id().to_string();
            let admin_title = p.title().to_string();
            let Some(admin_loc) = p.location.upgrade() else {
                log::error!("Admin '{p_id}' free floating in dimensionless void. Might need to eyeball their save file.");
                tell_user!(ctx.writer, "Something's awfully wrong. You're in the void. How did that happen, I have no clue… Check your save file.\n");
                ctx.state = ClientState::Logout;
                return;
            };
            let is_ghost = p.config.is_ghost;
            drop(p);
            (admin_title, is_ghost, admin_loc)
        };

        let Some((target_id, full_forced_cmd)) = ctx.args.split_once(' ') else {
            tell_user!(ctx.writer, "<c green>Usage:</c> force <target> <cmd> [<args>..]\n");
            return;
        };
        let (forced_cmd, _) = full_forced_cmd.split_once(' ').unwrap_or_else(||(full_forced_cmd, ""));

        if matches!((target_id.to_lowercase().as_str(), forced_cmd.to_lowercase().as_str()), ("force",_) | (_,"force")) {
            tell_user!(ctx.writer, "As much as 'use the force' is a meme, let's not do that, OK?\n");
            return;
        }

        // lets find out who to force:
        let forcetype = match target_id {
            "here"|"room" => ForceTarget::Room { id: admin_loc },
            "all" => ForceTarget::All,
            _ => {
                let w = ctx.world.read().await;
                let Some(target_arc) = w.players_by_id.get(target_id) else {
                    tell_user!(ctx.writer, "No such ID as '{}' online.\n");
                    return;
                };
                ForceTarget::Player { id: target_arc.clone() }
            }
        };

        ctx.out.broadcast.send(Broadcast::Force {
            command: full_forced_cmd.to_string(),
            who: forcetype,
            by: plr.clone().into(),
            delivery: if is_ghost {None} else {
                Some(format!("<c red>{admin_title}</c> <c yellow>issued a command which you had to heed to…</c>"))
            },
        }).ok();
    }
}
