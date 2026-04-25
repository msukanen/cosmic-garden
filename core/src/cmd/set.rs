//! 'set' a number of runtime variables.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, player::Player, show_help, show_help_if_needed, string::styling::Truthy, tell_user, thread::{SystemSignal, life::{TickType, sec_as_ticks}}, validate_access};

pub struct SetCommand;

#[async_trait]
impl Command for SetCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        show_help_if_needed!(ctx, "set");
        
        let (what, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        if args.is_empty() { show_help!(ctx, "set") }

        match what {
            "core_tick"|"core"|"core-tick" => set_core_tick(ctx, args).await,
            "battle_tick"|"battle"|"battle-tick" => set_battle_tick(ctx, args).await,
            "config" => set_config_val(ctx, args, &plr).await,
            _ => { show_help!(ctx, "set") }
        }
    }
}

///
/// Set core tick speed.
/// 
/// # Args
/// - `args` being a numeric value representing milliseconds.
async fn set_core_tick(ctx: &mut CommandCtx<'_>, args: &str) {
    if let Ok(msec) = args.parse::<f32>() {
        if msec < 10.0 {
            err_tell_user!(ctx.writer, "To prevent <c red>server fire hazard</c>, CG restricts core speed to max <c yellow>100Hz</c>.\n");
        } else if msec > 1000.0 {
            err_tell_user!(ctx.writer, "Eh, to tick slower than 1Hz? No thanks, try something faster…\n");
        }

        let usec = (msec * 1000.0 + msec.fract() * 1000.0).floor() as u64;
        let duration = Duration::from_micros(usec);
        ctx.out.life.send(SystemSignal::AlterTickRate { tick_type: TickType::Core, duration }).ok();
    }
    tell_user!(ctx.writer, "Speed must be a numeric millisecond value…\n\nCurrent core at <c yellow>{}Hz</c>", sec_as_ticks(1, TickType::Core, ctx.out).await);
}

///
/// Set battle tick speed.
/// 
/// # Args
/// - `args` being a numeric value representing milliseconds.
async fn set_battle_tick(ctx: &mut CommandCtx<'_>, args: &str) {
    if let Ok(msec) = args.parse::<f32>() {
        if msec < 10.0 {
            err_tell_user!(ctx.writer, "To prevent <c red>server fire hazard</c>, CG restricts core speed to max <c yellow>100Hz</c>.\n");
        } else if msec > 200.0 {
            err_tell_user!(ctx.writer, "Eh, to tick slower than 5Hz? No thanks, try something faster…\n");
        }

        let usec = (msec * 1000.0 + msec.fract() * 1000.0).floor() as u64;
        let duration = Duration::from_micros(usec);
        ctx.out.life.send(SystemSignal::AlterTickRate { tick_type: TickType::Battle, duration }).ok();
    }
    tell_user!(ctx.writer, "Speed must be a numeric millisecond value…\n\nCurrent battle speed at <c yellow>{}Hz</c>", sec_as_ticks(1, TickType::Battle, ctx.out).await);
}

///
/// Set a personal configuration variable.
/// 
async fn set_config_val(ctx: &mut CommandCtx<'_>, args: &str, plr: &Arc<RwLock<Player>>) {
    let config = plr.read().await.config.clone();
    let (var, args) = args.split_once(' ').unwrap_or((args, ""));
    if args.is_empty() {
        let mut out = String::from("<c cyan>Config values:</c>\n");
        out.push_str(&format!("   <c blue>*</c> show-<c yellow>id</c>: <c yellow>{}</c>\n", config.show_id));
        out.push_str(&format!("   <c blue>*</c> show-<c yellow>self</c>: <c yellow>{}</c>\n", config.show_self_in_room));
        out.push_str(&format!("   <c blue>*</c> <c yellow>ghost</c>: <c yellow>{}</c>\n", config.is_ghost));
        err_tell_user!(ctx.writer, "{}", out);
    }

    let tf = args.true_false();
    let mut p = plr.write().await;
    let cfg = &mut p.config;
    let which = match var {
        "id"|"show-id"|"show_id" => { cfg.show_id = tf; "show-id" }
        "self"|"show-self"|"show_self" => { cfg.show_self_in_room = tf; "show-self" }
        "ghost"|"invis" => { cfg.is_ghost = tf; "ghost" }
        _ => { drop(p); err_tell_user!(ctx.writer, "Accepted vars: <c yellow>id</c>, <c yellow>self</c>, <c yellow>ghost</c>.\n") }
    };
    drop(p);
    tell_user!(ctx.writer, "Variable <c cyan>{}</c> set to <c cyan>{}</c>.\n", which, tf);
}

#[cfg(test)]
mod cmd_set_tests {
    #[tokio::test]
    async fn set_config_val() {

    }
}
