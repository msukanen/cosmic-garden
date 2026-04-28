//! Clone things (not clowns though)!

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, identity::IdentityQuery, item::{container::Storage, ownership::{ItemSource, OwnedMut}}, roomloc_or_bust, show_help_if_needed, tell_user, thread::add_item_to_lnf, traits::Reflector, validate_access};

pub struct CloneCommand;

#[async_trait]
impl Command for CloneCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, true_builder);
        show_help_if_needed!(ctx, "clone");
        let loc = roomloc_or_bust!(plr);

        let p = plr.read().await;
        if let Some(item) = p.inventory.peek_at(ctx.args) {
            let mut new = item.deep_reflect();
            new.erase_owner_r();
            new.erase_last_user_r();
            let source = ItemSource::Admin { id: p.id().into() };
            let of = new.id().to_string();
            if let Err(e) = new.unify_source_r(&of, p.id(), &source) {
                log::error!("What gives? .deep_reflected copy refuses to accept {source:?}: {e:?}");
                err_tell_user!(ctx.writer, "Something awry with item source. Clowning aborted…\n");
            }
            drop(p);
            {
                let mut p = plr.write().await;
                if let Err(e) = p.inventory.try_insert(new) {
                    let item = e.extract_item();
                    drop(p);
                    tell_user!(ctx.writer, "Well dangit, no space in inventory. Slippery bugger that '{}'…\n", item.id());
                    let r_id = loc.read().await.id().to_string();
                    if let Err(e) = loc.write().await.try_insert(item) {
                        log::warn!("Room '{r_id}' full…");
                        tell_user!(ctx.writer, "… slippery enough to slip between the cracks of reality even!\n");
                        add_item_to_lnf(e).await;
                    }
                }
            }
        }
        // TODO entity clowing…
        else { tell_user!(ctx.writer, "TODO...\n") }
    }
}

#[cfg(test)]
mod cmd_clone_tests {
    use std::io::Cursor;

    use crate::{r#const::SMALL_ITEM, stabilize_threads, get_operational_mock_librarian, io::ClientState, item::{Item, container::Storage, ownership::Owner, weapon::{WeaponSize, WeaponSpec}}, string::Uuid, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn cmd_clone_knife() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let _ = get_operational_mock_librarian!(c,w);
        let c = c.out;
        stabilize_threads!();
        let item = Item::Weapon(WeaponSpec {
            id: "dinged-knifelike".re_uuid(),
            name: "A dinged knife".into(),
            desc: "A dingy dinged knife of dingyness.".into(),
            owner: Owner::blueprint(),
            size: SMALL_ITEM,
            weapon_size: WeaponSize::Small,
            base_dmg: 1.0,
        });
        p.write().await.inventory.try_insert(item).expect("Seriously? No space for a sm0l knife?");
        let state = ClientState::Playing { player: p.clone() };
    }
}
