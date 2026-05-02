//! Spawn something!

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx, inventory::InventoryCommand, look::LookCommand}, err_tell_user, identity::IdentityQuery, item::{StorageError, container::Storage}, player::Player, room::{Room, RoomPayload}, roomloc_or_bust, show_help, show_help_if_needed, tell_user, thread::{SystemSignal, add_item_to_lnf, librarian::get_item_blueprint, signal::SpawnType}, validate_access};

pub struct SpawnCommand;

#[async_trait]
impl Command for SpawnCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        show_help_if_needed!(ctx, "spawn");
        let loc = roomloc_or_bust!(plr);

        let (op, stem) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        if stem.is_empty() { show_help!(ctx, "spawn"); }

        match op.chars().nth(0) {
            Some(c) => match c {
                'e'|'E'|'m'|'M' => spawn_entity(ctx, loc, stem).await,
                'i'|'I' => spawn_item(ctx, loc, plr, stem).await,
                'l' => match stem.chars().nth(0) {
                    Some(c) => match c {
//                        'e' => list_entity(ctx, stem.split_once(' ').unwrap_or((stem, ""))),
//                        'i' => list_item(ctx, stem.split_once(' ').unwrap_or((stem, ""))),
                        _ => {
                            tell_user!(ctx.writer, "Um, list what now…?\n\n");
                            show_help!(ctx, "u spawn");
                        }
                    }
                    ,_=> unreachable!("stem != empty")
                }
                _ => {
                    tell_user!(ctx.writer, "'{}' is no known selector…\n\n", op);
                    show_help!(ctx, "u spawn");
                }
            }
            ,_=> unreachable!("ctx.args != empty")
        }
    }
}

/// Request an [Entity] to be spawned at `loc`.
async fn spawn_entity(ctx: &mut CommandCtx<'_>, loc: Arc<RwLock<Room>>, args: &str) {
    let (out, recv) = tokio::sync::oneshot::channel::<bool>();
    ctx.out.life.send(SystemSignal::Spawn {
        what: SpawnType::Mob { id: args.into() },
        room: RoomPayload::Arc(loc.clone()),
        reply: Some(out),
    }).ok();
    if let Ok(true) = recv.await {
        tell_user!(ctx.writer, "One '{}' has manifested!\n", args);
        LookCommand.exec({ctx.args = "q"; ctx}).await;
    } else {
        tell_user!(ctx.writer, "Mmm… that didn't seem to work. Maybe try some other entity name…?\n");
    }
}

/// Request an [Item] to be spawned either at `loc` or in caller's inventory.
async fn spawn_item(ctx: &mut CommandCtx<'_>, loc: Arc<RwLock<Room>>, plr: Arc<RwLock<Player>>, args: &str) {
    log::debug!("Spawn item called with: {args}");
    let (op, mut what) = args.split_once(' ').unwrap_or((args, ""));
    let mut here = false;
    match op {
        "here" => {
            if what.is_empty() {
                err_tell_user!(ctx.writer, "Well, ok, here — but which <c yellow>item</c>…?\n");
            }
            here = true;
        }
        _ => { what = args; }
    }
    
    if let Some(bp) = get_item_blueprint(what, &ctx.out).await {
        log::debug!("BP: {bp:?}");
        let bp_id = bp.id().to_string();
        let plr_item = if !here {
            plr.write().await.inventory.try_insert(bp)
        } else { Err(StorageError::NoSpace(bp)) };

        if let Err(e) = plr_item {
            log::debug!("Fell?");
            if let Err(e) = loc.write().await.try_insert(e.extract_item()) {
                add_item_to_lnf(e).await;
                err_tell_user!(ctx.writer, "Oops… you did spawn something{}, but it slipped between the planck cracks…\n",
                    if plr.read().await.config.show_id {format!(" <c gray>({bp_id})</c>")} else {"".into()});
            }
            err_tell_user!(ctx.writer, "You spawned '{}'{}\n", bp_id,
                if here {"."} else {" just fine, but bugger fell on the ground. Too heavy to carry…"}
            );
        }
        tell_user!(ctx.writer, "You're holding '{}' now!\n\n", bp_id);
        InventoryCommand.exec(ctx).await;
        return ;
    }
    err_tell_user!(ctx.writer, "Umm, no item blueprint found that'd match with '{}'", args);
}

#[cfg(test)]
mod cmd_spawn_tests {
    use std::io::Cursor;

    use crate::{cmd::{look::LookCommand, spawn::SpawnCommand}, ctx, get_operational_mock_librarian, get_operational_mock_life, stabilize_threads, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn spawn_knife() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),_) = get_operational_mock_world().await;
        let _ = get_operational_mock_librarian!(c,w);
        stabilize_threads!();
        let c = c.out;
        state = ctx!(state, SpawnCommand, "knife",s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        state = ctx!(state, SpawnCommand, "knif",s,c,w,p,|out:&str| out.contains("materialize"));
        state = ctx!(state, SpawnCommand, "f knif",s,c,w,p,|out:&str| out.contains("selector"));
        state = ctx!(state, SpawnCommand, "idemy knif",s,c,w,p,|out:&str| out.contains("no item blueprint"));
        _ = ctx!(state, SpawnCommand, "it knife",s,c,w,p,|out:&str| out.contains("carrying"));
    }

    #[tokio::test]
    async fn spawn_entity() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),_) = get_operational_mock_world().await;
        stabilize_threads!(250);// TODO investigate why this line is required here with full 'cargo test' and nowhere else…
        let _ = get_operational_mock_librarian!(c,w);
        let _ = get_operational_mock_life!(c,w);
        stabilize_threads!();
        let c = c.out;
        state = ctx!(state, SpawnCommand, "gobl",s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        state = ctx!(state, SpawnCommand, "gobl",s,c,w,p);//,|out:&str| out.contains("Entity or Item"));
        state = ctx!(state, SpawnCommand, "f gobl",s,c,w,p,|out:&str| out.contains("selector"));
        state = ctx!(state, SpawnCommand, "ent gobl",s,c,w,p,|out:&str| out.contains("try some other"));
        state = ctx!(state, LookCommand, "",s,c,w,p);
        _ = ctx!(state, SpawnCommand, "ent goblin",s,c,w,p,|out:&str| out.contains("manifested"));
    }
}
