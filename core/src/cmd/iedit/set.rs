//! Set some detail about an item in the IEDit.

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
            log::warn!("<what> now: {what}");
        loop {
            log::warn!("<what> now: {what}");
        match what {
            "heal" => match nuts {
                NutritionType::Heal { stat, drain } => 
                    // set nut heal <stat-type> <value>
                    // set nut heal <value-as-drain>  ;; to assign drain without bothering to re-type type
                    {
                        let (stat, value) = value.split_once(' ').unwrap_or((value, ""));
                        // if a float, value is 'drain'; + heals, - damages
                        if let Ok(v) = stat.parse::<f32>() {
                            *drain = v.clamp(-100.0, 100.0);// TODO drain - RECHECK min/max [14.04.2026]
                            tell_user!(ctx.writer, "Item 'drain' value set at: {}\n", *drain);
                            return ;
                        }

                        let stat = match stat.to_lowercase().as_str() {
                            "hp" => StatType::HP,
                            "mp" => StatType::MP,
                            "sn" => StatType::SN,
                            "san" => StatType::San,
                            _ => {tell_user!(ctx.writer, "Has to be one of the existing stat types: {}", StatType::display_list());return;}
                        };

                        if let Ok(v) = value.parse::<f32>() {
                            *drain = v.clamp(-100.0, 100.0);// TODO drain - RECHECK min/max [14.04.2026]
                            tell_user!(ctx.writer, "Item 'drain' value set at: {}\n", *drain);
                            return ;
                        }

                        // not a stat, not direct drain value...
                        tell_user!(ctx.writer, "Usage: set nut heal <stat> <value>\n       set nut heal <drain>\n");
                        return ;
                    },

                NutritionType::NotEdible =>
                    {
                        *nuts = NutritionType::Heal { stat: crate::mob::StatType::HP, drain: 0.0 };
                        continue;
                    }
                },
            _ => {tell_user!(ctx.writer, "Unfortunately only 'heal' is currently available.\n");return;}
        }
        }
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

#[cfg(test)]
mod cmd_iedit_set_tests {
    use crate::{cmd::{Command, CommandCtx, iedit::{IeditCommand, desc::DescCommand, iex::IexCommand, set::SetCommand}}, ctx, io::ClientState, util::access::Access, world::world_tests::get_operational_mock_world};
    
    #[tokio::test]
    async fn iedit_set_something_on_primordial() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut mock_sock = std::io::Cursor::new(&mut buffer);
        let (tx, _) = tokio::sync::broadcast::channel::<crate::Broadcast>(16);
        let (world, plr) = get_operational_mock_world().await;
        plr.write().await.access = Access::Builder;
        
        log::debug!("Debugging the Kobolds away!");
        ctx!(IeditCommand, "apple", mock_sock, tx, world, plr);
        ctx!(IexCommand, "", mock_sock, tx, world, plr);
        ctx!(SetCommand, "pot cons", mock_sock, tx, world, plr);
        ctx!(SetCommand, "nut inedible", mock_sock, tx, world, plr);
        ctx!(SetCommand, "nut heal hp 10.0", mock_sock, tx, world, plr);
        ctx!(DescCommand, "=It's not soup anymore. It's ...", mock_sock, tx, world, plr);
        ctx!(DescCommand, "+3 ...", mock_sock, tx, world, plr);
        ctx!(DescCommand, "+5 ... an apple!", mock_sock, tx, world, plr);
        ctx!(IexCommand, "", mock_sock, tx, world, plr);
        ctx!(DescCommand, "", mock_sock, tx, world, plr);
    }
}