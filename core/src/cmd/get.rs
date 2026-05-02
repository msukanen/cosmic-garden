//! Get something from ground…

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, identity::IdentityQuery, item::{Item, container::Storage}, player::Player, player_or_bust, room::Room, roomloc_or_bust, show_help_if_needed, tell_user, thread::add_item_to_lnf, util::activity::ActionWeight};

pub struct GetCommand;

#[async_trait]
impl Command for GetCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        show_help_if_needed!(ctx, "get");
        let p_loc = roomloc_or_bust!(plr);

        let (mut what, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));

        match (what, args) {
            ("all", "") => err_tell_user!(ctx.writer, "Right, how about no? I refuse to vacuum up the whole world…\n"),
            ("all", _) => {
                let Some(from) = p_loc.read().await.find_id_by_name(args) else {
                    err_tell_user!(ctx.writer, "No such thing here…\n");
                };

                bulk_transfer(ctx, plr.clone(), p_loc.clone(), &from).await;
                return
            }
            _ => what = ctx.args,
        }

        let thing_id = {
            let r = p_loc.read().await;
            let Some(thing) = r.find_id_by_name(what) else {
                err_tell_user!(ctx.writer, "No such thing here…\n");
            };
            thing
        };

        if let Some(Item::Corpse{..}) = p_loc.read().await.peek_at(&thing_id) {
            err_tell_user!(ctx.writer, "Maybe better leave that for an undertaker or something…\n");
        }

        let Some(item) = p_loc.write().await.take(&thing_id) else {
            tell_user!(ctx.writer, "It's stuck?\n");
            return;
        };

        let item_name = item.title().to_string();
        let act_w = item.required_space();
        let item_err = {
            let mut lock = plr.write().await;
            let Err(item_err) = lock.inventory.try_insert(item) else {
                tell_user!(ctx.writer, "You nab '{}'!\n", item_name);
                lock.act(plr.clone(), &ctx.out, ActionWeight::ItemTransfer { count: act_w as usize }).await;
                return;
            };
            drop(lock);

            // bugger, no space in inventory, lets put it back...
            let Err(item_err) = p_loc.write().await.try_insert(item_err.into()) else {
                tell_user!(ctx.writer, "Way too big or heavy. You set it back before you break your back.\n");
                return;
            };

            item_err
        };
        add_item_to_lnf(item_err).await;
        tell_user!(ctx.writer, "… the world is being weird …\n");
    }
}

/// Bulk transfer everything within `from` to `plr` inventory who is at `room`.
async fn bulk_transfer(ctx: &mut CommandCtx<'_>, plr: Arc<RwLock<Player>>, room: Arc<RwLock<Room>>, from: &str) {
    if let Some(src) = room.write().await.peek_at_mut(from) {
        if !matches!(*src, Item::Container(_)|Item::Corpse{..}) {
            err_tell_user!(ctx.writer, "'{}' doesn't contain anything. Did you mean to pick it insted?\n", from)
        }
        let Some(items) = src.eject_all() else {
            err_tell_user!(ctx.writer, "Bummer, '{}' seems to be empty!\n", from)
        };
        let mut not_taken = vec![];
        let mut taken_names = vec![];
        // Shove the things into player's inventory, if we can.
        {
            let mut p = plr.write().await;
            for item in items {
                let name = item.title().to_string();
                if let Err(e) = p.inventory.try_insert(item) {
                    not_taken.push(e.extract_item());
                    continue;
                }
                taken_names.push(name);
            }
        }
        let ntl = not_taken.len();
        for item in not_taken {
            if let Err(e) = src.try_insert(item) {
                log::error!("Something fishy with {src:?} - cannot insert - {e:?}");
                add_item_to_lnf(e).await;
            }
        }
        
        if taken_names.is_empty() {
            let (a,s,ait) = match ntl {
                1 => ("a ","",""),
                _ => ("", "s", "any of ")
            };
            err_tell_user!(ctx.writer, "There's {}thing{}, but you don't seem to be able to carry {}it…\n", a,s,ait);
        }
    }
}

#[cfg(test)]
mod cmd_get_tests {
    use std::io::Cursor;

    use crate::{cmd::{attack::AttackCommand, look::LookCommand}, ctx, get_operational_mock_librarian, get_operational_mock_life, room::RoomPayload, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn get_all() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(mut state,p),_) = get_operational_mock_world().await;
        get_operational_mock_librarian!(c,w);
        get_operational_mock_life!(c,w);
        stabilize_threads!();
        let c = c.out;
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: RoomPayload::Id("r-1".into()), reply: None }).ok();
        stabilize_threads!(25);
        state = ctx!(sup true, state, AttackCommand, "goblin",s,c,w,p);
        // let combat roll a moment…
        stabilize_threads!(2000);
        state = ctx!(state, LookCommand,"",s,c,w,p);
    }
}
