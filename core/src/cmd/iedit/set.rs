//! Set some detail about an item in the IEDit.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_iedit_buffer_inaccessible, identity::{IdentityMut, IdentityQuery}, item::{Item, ItemizedMut, consumable::NutritionType, container::{StorageMut, specs::StorageSpace}, primordial::PotentialItemType}, mob::StatType, tell_user, validate_access};

pub struct SetCommand;

macro_rules! no_can_do {
    ($ctx:ident, $what:expr) => {{
        tell_user!($ctx.writer, "Item's {} is immutable, sorry.\n", $what);return;
    }};
}

#[async_trait]
impl Command for SetCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let p_id = {
            let p = plr.read().await;
            let p_id = p.id().to_string();
            p_id
        };

        let (field, value) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));

        if value.is_empty() {
            tell_user!(ctx.writer,r#"Valid settable fields:
 * title
 * size
 * max_space / max
 * potential / pot

For description, use 'desc' command instead.
"#);
            return;
        }

        let mut p = plr.write().await;
        let Some(ed) = p.iedit_buffer.as_mut() else {
            err_iedit_buffer_inaccessible!(ctx,p,p_id);
        };

        match field {
            "title" => {
                ed.set_title(value);
                tell_user!(ctx.writer, "Title set to: {}\n", value);
            },

            "size" => size_does_matter(ctx, ed, value).await,
            "max_space"|
            "max" => do_max(ctx, ed, value).await,
            "potential"|
            "pot" => do_harry_pot(ctx, ed, value).await,
            "desc" => tell_user!(ctx.writer, "Well, there's the <c yellow>desc</c> command set for that…\n"),
            "use"|"uses" => do_uses(ctx, ed, value).await,
            "nut"|"nutrition" => go_nuts(ctx, ed, value).await,
            _ => tell_user!(ctx.writer, "No such field to alter, and I can't just whip up new fields out of nothing…\n")
        }
    }
}

/// Deal with item 'uses'.
async fn do_uses(ctx: &mut CommandCtx<'_>, ed: &mut Item, value: &str) {
    if let Some(ref mut use_access) = match ed {
        Item::Consumable(v) => v.uses,
        Item::Primordial(v) => v.uses,
        _ => None
    } {
        *use_access = match value.parse::<i32>() {
            Err(_) => { tell_user!(ctx.writer, "Seriously…? How about no.\n");return; },
            Ok(v) if v > 1000 => usize::MAX,
            Ok(v) if v < 0 => { tell_user!(ctx.writer, "\"Uses left\" doesn't work like that, truly…\n"); return;}
            Ok(v) => v as usize
        };
    }
}

/// Deal with item 'potential'.
async fn do_harry_pot(ctx: &mut CommandCtx<'_>, ed: &mut Item, value: &str) {
    if let Item::Primordial(v) = ed {
        let err = PotentialItemType::from(value);
        if err.is_err() {
            tell_user!(ctx.writer, "That doesn't work, the variants are: {}\n", err.err().unwrap());
            return;
        };
        let pot = err.ok().unwrap();
        v.set_potential(pot.clone());
        tell_user!(ctx.writer, "Item potential set as '{}'\n", pot);
    }
    else {
        no_can_do!(ctx, "potential");
    }
}

/// Deal with item 'max_space'.
async fn do_max(ctx: &mut CommandCtx<'_>, ed: &mut Item, value: &str) {
    if matches!(ed, Item::Container(_)|Item::Primordial(_)) {
    if let Ok(sz) = value.parse::<StorageSpace>() {
        if !ed.set_max_space(sz) {
            tell_user!(ctx.writer, "Ugh, too much stuff in there…\nMight consider <c yellow>'weave'</c> and put them things elsewhere first.\n");
            return;
        }
        tell_user!(ctx.writer, "Max space set to: {}\n", sz);
    }} else {
        no_can_do!(ctx, "max_space");
    }
}

/// Size does matter, at times… Deal with item 'size'.
async fn size_does_matter(ctx: &mut CommandCtx<'_>, ed: &mut Item, value: &str) {
    if let Ok(sz) = value.parse::<StorageSpace>() {
        if !ed.set_size(sz) {
            tell_user!(ctx.writer, "That item's size is immutable, sorry…\n");
            return;
        }
        tell_user!(ctx.writer, "Size set to: {}\n", sz);
    }
}

/// Deal with item 'nutrition'.
async fn go_nuts(ctx: &mut CommandCtx<'_>, ed: &mut Item, value: &str) {
    // Is it chewy or not?
    if !matches!(ed, Item::Consumable(_)|Item::Primordial(_)) {
        no_can_do!(ctx, "nutrition");
    }

    // Actually set some `value` after exact `what` has been determined.
    async fn go_really_nuts(ctx: &mut CommandCtx<'_>, nuts: &mut NutritionType, what: &str, value: &str) {
        loop {
        match what {
            "heal" => match nuts {
                NutritionType::NotEdible  => {*nuts = NutritionType::Heal { stat: StatType::HP, drain: 0.0 }; continue; },
                NutritionType::Heal { stat, drain }=> {
                    *nuts = NutritionType::MultiHeal { stat_n_drain: {
                        let mut m = HashMap::new();
                        m.insert(stat.clone(), *drain);
                        m
                    } };
                    continue;
                },
                NutritionType::MultiHeal { stat_n_drain } =>
                    // set nut multi <stat-type> <value>
                    {
                        let (maybe_rm, arg) = value.split_once(' ').unwrap_or((value, ""));
                        match maybe_rm {
                            "rm"|"del"|"remove"|"delete"|"wipe"|"erase"|"reject" => {
                                let Ok(stat_type) = StatType::try_from(arg) else {
                                    if arg.is_empty() {
                                        tell_user!(ctx.writer,
                                            "You need to specify a stat, one of <c yellow>{}</c>\n<c green>Usage:</c> set nut multi rm <c cyan><stat></c>\n",
                                            StatType::display_list()
                                        );
                                    } else {
                                        tell_user!(ctx.writer, "Well, '{}' is no stat type I'd recognize…\n", arg);
                                    }
                                    return ;
                                };
                                stat_n_drain.remove(&stat_type);
                                if stat_n_drain.is_empty() {
                                    *nuts = NutritionType::NotEdible;
                                    tell_user!(ctx.writer, "It's not very edible anymore…\n");
                                    return ;
                                } else if stat_n_drain.len() == 1 {
                                    // fall back to Heal
                                    let (s,d) = stat_n_drain.iter().next().unwrap();
                                    *nuts = NutritionType::Heal { stat: s.clone(), drain: *d };
                                }
                                tell_user!(ctx.writer, "Item set as {}\n", *nuts);
                                return ;
                            }
                            _ => ()
                        }

                        match parse_stat_n_val(value) {
                            Err(e) => tell_user!(ctx.writer, "{}\n", e),
                            Ok((None, None)) => {
                                *nuts = NutritionType::NotEdible;
                                tell_user!(ctx.writer, "That is *definitely* inedible…!\n");
                            },
                            Ok((Some(s), d)) => {
                                let d = d.expect(&format!("LOGIC: parse_stat_n_val failure in <f32> parsing!: {value}")); // fix parse_stat_n_val if this blows up…
                                if d.abs() < 0.001 {
                                    // effectively zero - erase instead of zeroing.
                                    stat_n_drain.remove(&s);
                                    if stat_n_drain.is_empty() {
                                        *nuts = NutritionType::NotEdible;
                                        tell_user!(ctx.writer, "It's not very edible anymore…\n");
                                        return ;
                                    } else if stat_n_drain.len() == 1 {
                                        // fall back to Heal
                                        let (s,d) = stat_n_drain.iter().next().unwrap();
                                        *nuts = NutritionType::Heal { stat: s.clone(), drain: *d };
                                    }
                                    tell_user!(ctx.writer, "Item set as {}\n", *nuts);
                                    return ;
                                }
                                stat_n_drain.insert(s, d);
                                if stat_n_drain.len() == 1 {
                                    // fall back to Heal
                                    let (s,d) = stat_n_drain.iter().next().unwrap();
                                    *nuts = NutritionType::Heal { stat: s.clone(), drain: *d };
                                }
                                tell_user!(ctx.writer, "Item set as: {}\n", *nuts);
                            },
                            _ => tell_user!(ctx.writer, "<c green>Usage:</c> set nut multi <stat> <val>\n       set nut multi rm <stat>\n")
                        }

                        return ;
                    },
                }

            _ => {tell_user!(ctx.writer, "Unfortunately only 'heal' is currently available.\n");return;}

        }}
    }

    // set nut <..>
    let (sub, value) = value.split_once(' ').unwrap_or((value, ""));
    if match sub {
        // set nut inedible
        "inedible"|"na"|"no"|"eww"|"nope"|"awful" => true,_=> false
    } {
        match ed {
            Item::Consumable(v) => v.nutrition = NutritionType::NotEdible,
            Item::Primordial(v) => v.nutrition = None,
            _ => { tell_user!(ctx.writer, "It wasn't very nutritional anyway…\n"); return ; }
        }
        tell_user!(ctx.writer, "Item set as 'inedible'.\n");
        return;
    }

    // set nut <what> <value>
    if value.is_empty() {
        tell_user!(ctx.writer, "Some sort of a value is needed.\n");
        return;
    }

    match ed {
        Item::Consumable(v) => go_really_nuts(ctx, &mut v.nutrition, sub, value).await,
        Item::Primordial(v) => {
            let mut n = v.nutrition.clone().unwrap_or_default();
            go_really_nuts(ctx, &mut n, sub, value).await;
            v.nutrition = if matches!(n, NutritionType::NotEdible) { None } else { Some(n) }
        },
        _ => unreachable!("This should not happen…")
    }
}

fn parse_stat_n_val(input: &str) -> Result<(Option<StatType>, Option<f32>), String> {
    let (r#type, value) = input.split_once(' ').unwrap_or((input, ""));
    // if a float, value is 'drain'; + heals, - damages
    if let Ok(v) = r#type.parse::<f32>() {
        return Ok((None, Some(v.clamp(-100.0, 100.0))));// TODO drain - RECHECK min/max [14.04.2026]
    }

    let stat_type = match r#type.to_lowercase().as_str() {
        "hp" => StatType::HP,
        "mp" => StatType::MP,
        "sn" => StatType::SN,
        "san" => StatType::San,
        _ => return Err(format!("Has to be one of the existing stat types: {}", StatType::display_list()))
    };

    let Ok(v) = value.parse::<f32>() else {
        return Err(format!("Drain on {stat_type} has to be a suitable numeric value…"));
    };

    Ok((stat_type.into(), v.clamp(-100.0, 100.0).into()))
}

#[cfg(test)]
mod cmd_iedit_set_tests {
    use crate::{cmd::iedit::{IeditCommand, desc::DescCommand, iex::IexCommand, set::SetCommand, weave::WeaveCommand}, ctx, io::ClientState, util::access::Access, world::world_tests::get_operational_mock_world};
    
    #[tokio::test]
    async fn iedit_set_something_on_primordial() {
        let mut b: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut b);
        let (w, c, p) = get_operational_mock_world().await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, IeditCommand, "apple", s, c.out, w, p, |out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, IeditCommand, "apple", s, c.out, w, p);
        let state = ctx!(state, IexCommand, "", s, c.out, w, p);
        let state = ctx!(state, SetCommand, "pot cons", s, c.out, w, p);
        let state = ctx!(state, SetCommand, "nut inedible", s, c.out, w, p);
        let state = ctx!(state, SetCommand, "nut heal hp 10.0", s, c.out, w, p);
        let state = ctx!(state, DescCommand, "=It's not soup anymore. It's ...", s, c.out, w, p);
        let state = ctx!(state, DescCommand, "+3 ...", s, c.out, w, p);
        let state = ctx!(state, DescCommand, "+5 ... an apple!", s, c.out, w, p);
        let state = ctx!(state, IexCommand, "", s, c.out, w, p);
        let _ = ctx!(state, DescCommand, "", s, c.out, w, p);
    }

    #[tokio::test]
    async fn iedit_crank_something_on_primordial() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut buffer);
        let (w, c, plr) = get_operational_mock_world().await;
        let state = ClientState::Playing { player: plr.clone() };
        let state = ctx!(state, IeditCommand, "apple", s, c.out, w, plr,|out:&str| out.contains("Huh?"));
        plr.write().await.access = Access::Builder;
        let state = ctx!(state, IeditCommand, "apple", s, c.out, w, plr);
        let state = ctx!(state, IexCommand, "", s, c.out, w, plr);
        let state = ctx!(state, SetCommand, "pot cons", s, c.out, w, plr);
        let state = ctx!(state, SetCommand, "nut inedible", s, c.out, w, plr);
        let state = ctx!(state, SetCommand, "nut heal hp 10.0", s, c.out, w, plr);
        let state = ctx!(state, DescCommand, "=It's not soup anymore. It's ...", s, c.out, w, plr);
        let state = ctx!(state, DescCommand, "+3 ...", s, c.out, w, plr);
        let state = ctx!(state, DescCommand, "+5 ... an apple!", s, c.out, w, plr);
        let state = ctx!(state, IexCommand, "", s, c.out, w, plr);
        let _ = ctx!(state, DescCommand, "", s, c.out, w, plr);
    }

    #[tokio::test]
    async fn iedit_set_multi() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut s = std::io::Cursor::new(&mut buffer);
        let (w, c, p) = get_operational_mock_world().await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(state, IeditCommand, "apple", s,c.out,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, IeditCommand, "apple", s,c.out,w,p,|out:&str| out.contains("new"));
        let state = ctx!(state, IexCommand, "", s, c.out, w, p);
        let state = ctx!(state, SetCommand, "nut heal", s,c.out,w,p,|out:&str| out.contains("value is"));   // fail state
        let state = ctx!(state, SetCommand, "nut heal 5", s,c.out,w,p,|out:&str| out.contains("Usage"));           // fail state
        let state = ctx!(state, IexCommand, "", s, c.out, w, p,|out:&str| out.contains("Heal(HP"));               // ok
        let state = ctx!(state, SetCommand, "nut heal 1.0",s,c.out,w,p,|out:&str| out.contains("Usage:"));        // fail state
        let state = ctx!(state, SetCommand, "nut heal hp", s,c.out,w,p,|out:&str| out.contains("has to be"));     // fail state
        let state = ctx!(state, SetCommand, "nut heal hp 0", s,c.out,w,p,|out:&str| out.contains("edibl"));       // back in inedbile
        let state = ctx!(state, IexCommand, "", s, c.out, w, p,|out:&str| out.contains("type: <n/a>"));
        let state = ctx!(state, SetCommand, "nut heal hp 1", s,c.out,w,p,|out:&str| out.contains(": Heal"));       // ok
        let state = ctx!(state, SetCommand, "nut heal sn -0.5", s,c.out,w,p,|out:&str| out.contains(": Multi"));
        let state = ctx!(state, IexCommand, "",s, c.out, w, p,|out:&str| out.contains("SN -0.50") && out.contains("HP +"));
        let state = ctx!(state, SetCommand, "nut heal rm",s,c.out,w,p,|out:&str| out.contains("HP, MP"));         // fail state
        let state = ctx!(state, SetCommand, "nut heal rm hp",s,c.out,w,p,|out:&str| out.contains("Heal(SN -0.50)"));  // ok - fall back to Heal{..}
        let state = ctx!(state, SetCommand, "nut heal sn 0.25", s,c.out,w,p,|out:&str| out.contains("Heal(SN +0.25)"));  // ok
        let state = ctx!(state, SetCommand, "nut heal hp 1",s,c.out,w,p,|out:&str| out.contains("SN +0.25") && out.contains("HP +1.0"));//ok
        let state = ctx!(state, WeaveCommand, "",s, c.out, w, p,|out:&str| out.contains("created something"));
        let state = ctx!(state, IeditCommand, "apple", s,c.out,w,p,|out:&str| out.contains("what it's"));
        let state = ctx!(state, IexCommand, "",s, c.out, w, p,|out:&str| out.contains("PrimordialItem"));         // aw poo, "forgot" to set cons
        let state = ctx!(state, SetCommand, "pot cons",s,c.out,w,p);
        let state = ctx!(state, WeaveCommand, "",s, c.out, w, p,|out:&str| out.contains("created something"));
        let state = ctx!(state, IeditCommand, "apple", s,c.out,w,p,|out:&str| out.contains("what it's"));
        let _ = ctx!(state, IexCommand, "",s, c.out, w, p,|out:&str| out.contains("Consumable"));             // yay, an edible apple, sort of.
    }
}
