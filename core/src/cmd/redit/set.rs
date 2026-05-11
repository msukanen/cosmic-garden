//! REdit 'set' for various settables…

use std::sync::RwLock;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, room::{RoomArc, environ::{GRAVITY_ANOMALY_HIGH_H, GRAVITY_ANOMALY_LOW_H, SPECIAL_ENVIRONMENT_CORROSIVE, SPECIAL_ENVIRONMENT_FOGGED_VISIBILITY, SPECIAL_ENVIRONMENT_FREEZER, SPECIAL_ENVIRONMENT_GAS_TRAP, SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY, SPECIAL_ENVIRONMENT_INFERNO, SPECIAL_ENVIRONMENT_LOUD, SPECIAL_ENVIRONMENT_OBSTRUCTED_VISIBILITY, SPECIAL_ENVIRONMENT_STINKY, SPECIAL_ENVIRONMENT_TOXIC}}, roomloc_or_bust, show_help, show_help_if_needed, tell_user, validate_access, validate_editor_mode};

pub struct SetCommand;

#[async_trait]
impl Command for SetCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let loc = roomloc_or_bust!(plr);
        validate_editor_mode!(ctx, "REdit");
        show_help_if_needed!(ctx, "set");

        // s - self.special_environment
        // m - self.memory_fog = source.memory_fog;
        // t - self.terrain = source.terrain;
        // r - self.room_type = source.room_type;

        let (mut op, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        if op.is_empty() { show_help!(ctx, "u set"); }
        match &op[..1] {
            "s"|"S" => set_spec_env(ctx, &loc, args).await,
            "m"|"M" => set_mem_fog(ctx, &loc, args).await,
            "t"|"T" => set_terrain(ctx, &loc, args).await,
            "r"|"R" => set_room_type(ctx, &loc, args).await,
            _ => {
                tell_user!(ctx.writer, "Uhm, '{}' is not recognized as an operator for <x cmd>set</x>…", op);
                show_help!(ctx, "u set");
            }
        }
    }
}

/// Set [special environment][SpecialEnvironment], if possible.
async fn set_spec_env(ctx: &mut CommandCtx<'_>, room: &RoomArc, args: &str) {
    let (env, args) = args.split_once(' ').unwrap_or((args, ""));
    if env.len() < 2 {
        err_tell_user!(ctx.writer, "Well… '{}' is a bit too ambiguous. Be more explicit a bit.\n", env);
    }
    room.write().await.set_special_env_bitmask(
    match &env[..2] {
        // corrosive|acidic
        "co"|"CO"|
        "ac"|"AC" => SPECIAL_ENVIRONMENT_CORROSIVE,

        // gas trap
        "ga"|"GA"|
        "gt"|"GT" => SPECIAL_ENVIRONMENT_GAS_TRAP,

        // toxic
        "to"|"TO"|
        "tx"|"TX" => SPECIAL_ENVIRONMENT_TOXIC,

        // fogged
        "fo"|"FO" => SPECIAL_ENVIRONMENT_FOGGED_VISIBILITY,
        
        // freezer
        "fr"|"FR" => SPECIAL_ENVIRONMENT_FREEZER,
        
        // inferno|very hot (treated alike…)
        "in"|"IN"|
        "vh"|"VH" => SPECIAL_ENVIRONMENT_INFERNO,
        
        // obstructed visibility
        "ob"|"OB"|
        "ov"|"OV" => SPECIAL_ENVIRONMENT_OBSTRUCTED_VISIBILITY,

        // high-g
        "hi"|"HI"|
        "hg"|"HG" => SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY | GRAVITY_ANOMALY_HIGH_H,
        // low-g
        "lg"|"LG" => SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY | GRAVITY_ANOMALY_LOW_H,

        // loud (or low-g)
        "lo"|"LO" =>
            if env.len() > 2 && matches!(&env[..2], "low"|"LOW"|"Low") {
                SPECIAL_ENVIRONMENT_GRAVITY_ANOMALY | GRAVITY_ANOMALY_LOW_H
            } else {
                SPECIAL_ENVIRONMENT_LOUD
            },

        "sm"|"SM"|
        "st"|"ST" => SPECIAL_ENVIRONMENT_STINKY,

        _ => {
            tell_user!(ctx.writer, "'{}' doesn't represent any known environment…\n", env);
            show_help!(ctx, "q special-environment");
        }
    }, false).ok();
}

/// Set [memory fog][MemoryFogType]…
async fn set_mem_fog(ctx: &mut CommandCtx<'_>, room: &RoomArc, args: &str) {

}

/// Set the major [terrain][Terrain] type…
async fn set_terrain(ctx: &mut CommandCtx<'_>, room: &RoomArc, args: &str) {

}

/// Set a [Room]'s general [type][RoomType].
async fn set_room_type(ctx: &mut CommandCtx<'_>, room: &RoomArc, args: &str) {

}
