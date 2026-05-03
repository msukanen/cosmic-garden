//! 'set' [Entity] stats, etc.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, item::weapon::WeaponSize, mob::{Stat, StatType, StatValue, core::{EntitySize, EntitySizeError}, faction::{EntityFaction, EntityFactionError, FactionMut, Factioned}, traits::{Mob, MobMut}}, show_help, show_help_if_needed, tell_user, validate_access, validate_editor_mode};

pub struct SetCommand;

#[async_trait]
impl Command for SetCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        validate_editor_mode!(ctx, "MEdit");
        show_help_if_needed!(ctx, "set");

        let (what, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        if args.is_empty() {
            show_help!(ctx, "q set");
        }
        if let Some(ed) = &mut plr.write().await.medit_buffer {
            // a stat?
            if let Ok(stat_type) = StatType::try_from(what) {
                match set_stat_val_n_max(ed.stat_mut(&stat_type), args) {
                    Err(StatValErr::UsageReq) => show_help!(ctx, "q set"),
                    Err(StatValErr::NoSuchVal(v)) => err_tell_user!(ctx.writer, "{}\n", v),
                    Ok(()) => {
                        tell_user!(ctx.writer, "Stat set: <c cyan>{}</c>\n", ed.stat_mut(&stat_type));
                        return ;
                    }
                }
            }
            // safe to unwrap as ctx.args cannot be empty down here.
            match what.chars().nth(0).unwrap() {
                'f'|'F' => if let Err(e) = set_faction(ed.faction_mut(), args) {
                    err_tell_user!(ctx.writer, "Uh… {}\n", e);
                } else { tell_user!(ctx.writer, "Faction set: <c cyan>{}</c>\n", ed.faction())},
                'm'|'M'|
                'w'|'W' => if let Err(e) = set_max_weapon_size(ed.max_weapon_size_mut(), args) {
                    err_tell_user!(ctx.writer, "Uh… {}\n", e);
                } else { tell_user!(ctx.writer, "Max weapon size set: <c cyan>{}</c>\n", ed.max_weapon_size())},
                's'|'S' => if let Err(e) = set_size(ed.size_mut(), args) {
                    err_tell_user!(ctx.writer, "Uh… {}\n", e);
                } else { tell_user!(ctx.writer, "Size set: <c cyan>{}</c>\n", ed.size())},
                
                _ => err_tell_user!(ctx.writer, "Known operators are: <c yellow>f</c>action, <c yellow>m</c>ax <c yellow>w</c>eapon size, <c yellow>s</c>ize.")
            }
        } else {
            log::error!("Builder's medit_buffer evaporated?!");
            err_tell_user!(ctx.writer, "There should've been something in your editor buffer, but… Huh.\n");
        }
    }
}

enum StatValErr {
    NoSuchVal(String),
    UsageReq
}

/// Set [Stat] current (and max) value(s).
fn set_stat_val_n_max(stat: &mut Stat, args: &str) -> Result<(), StatValErr> {
    let (val, remainder) = args.split_once(' ').unwrap_or((args, ""));
    let Ok(v) = val.parse::<StatValue>() else {
        return Err(StatValErr::NoSuchVal(format!("Value needs to be numeric… '{val}' is no such value.")));
    };
    // set .. v
    stat.set_curr(v);
    if remainder.is_empty() { return Ok(()) }

    let (max, val) = remainder.split_once(' ').unwrap_or((remainder, ""));
    match max {
        // set .. <v> max vmax
        "max" => {
            let Ok(vmax) = val.parse::<StatValue>() else {
                return Err(StatValErr::NoSuchVal(format!("Current set as {v}, but max '{}' was not a numeric value…", val)));
            };
            stat.set_max(vmax);
            Ok(())
        }

        _ => Err(StatValErr::UsageReq)
    }
}

/// Set [Entity]'s [faction][EntityFaction].
fn set_faction(fact: &mut EntityFaction, args: &str) -> Result<(), EntityFactionError> {
    match EntityFaction::try_from(args) {
        Ok(f) => { *fact = f; Ok(()) }
        Err(e) => Err(e)
    }
}

/// Set [Entity]'s max weapon size.
fn set_max_weapon_size(mxwsz: &mut WeaponSize, args: &str) -> Result<(), String> {
    match WeaponSize::try_from(args) {
        Ok(sz) => { *mxwsz = sz; Ok(()) }
        Err(e) => Err(e)
    }
}

/// Set [Entity]'s size/stature.
fn set_size(ent_sz: &mut EntitySize, args: &str) -> Result<(), EntitySizeError> {
    match EntitySize::try_from(args) {
        Ok(sz) => { *ent_sz = sz; Ok(()) }
        Err(e) => Err(e)
    }
}

#[cfg(test)]
mod medit_set_tests {
    use std::io::Cursor;

    use crate::{cmd::medit::{MeditCommand, rename::RenameCommand, set::SetCommand}, ctx, get_operational_mock_librarian, get_operational_mock_life, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn set_test() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let _ = get_operational_mock_life!(c,w);
        let _ = get_operational_mock_librarian!(c,w);
        stabilize_threads!();
        let c = c.out;
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(25);
        let state = ctx!(sup state, MeditCommand, "goblin", s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(sup state, MeditCommand, "goblin", s,c,w,|out:&str| out.contains("nvoked"));
        let state = ctx!(sup state, RenameCommand, "Morg-Gluglug",s,c,w,|out:&str| out.contains("renamed"));
        //let state = ctx!(state, IexCommand, "",s,c,w);
        let state = ctx!(sup state, SetCommand, "h", s,c,w,|out:&str| out.contains("various aspects"));
        // set size ..
        let state = ctx!(sup state, SetCommand, "s u", s,c,w,|out:&str| out.contains("Uh…"));
        let state = ctx!(sup state, SetCommand, "s v", s,c,w,|out:&str| out.contains("Size set"));
        let state = ctx!(sup state, SetCommand, "s t", s,c,w,|out:&str| out.contains("Size set"));
        let state = ctx!(sup state, SetCommand, "s m", s,c,w,|out:&str| out.contains("Size set"));
        let state = ctx!(sup state, SetCommand, "s L", s,c,w,|out:&str| out.contains("Size set"));
        let state = ctx!(sup state, SetCommand, "s Googolplex", s,c,w,|out:&str| out.contains("Size set"));
        let state = ctx!(sup state, SetCommand, "s Sm0l", s,c,w,|out:&str| out.contains("Size set"));
        // weapon size ..
        let state = ctx!(sup state, SetCommand, "w u", s,c,w,|out:&str| out.contains("Uh…"));
        let state = ctx!(sup state, SetCommand, "w v", s,c,w,|out:&str| out.contains("not recognized"));
        let state = ctx!(sup state, SetCommand, "w t", s,c,w,|out:&str| out.contains("eapon size set"));
        let state = ctx!(sup state, SetCommand, "w m", s,c,w,|out:&str| out.contains("eapon size set"));
        let state = ctx!(sup state, SetCommand, "w L", s,c,w,|out:&str| out.contains("eapon size set"));
        let state = ctx!(sup state, SetCommand, "w Googolplex", s,c,w,|out:&str| out.contains("not recognized"));
        let state = ctx!(sup state, SetCommand, "w Sm0l", s,c,w,|out:&str| out.contains("pon size set"));
        // faction ..
        let state = ctx!(sup state, SetCommand, "f u", s,c,w,|out:&str| out.contains("Uh…"));
        let state = ctx!(sup state, SetCommand, "f p", s,c,w,|out:&str| out.contains("Uh…"));
        let state = ctx!(sup state, SetCommand, "f h", s,c,w,|out:&str| out.contains("HOSTILE"));
        let state = ctx!(sup state, SetCommand, "f f", s,c,w,|out:&str| out.contains("friendly"));
        let state = ctx!(sup state, SetCommand, "f Googolplex", s,c,w,|out:&str| out.contains("Guard"));
        let _ = ctx!(sup state, SetCommand, "f n", s,c,w,|out:&str| out.contains("neutral"));
    }
}
