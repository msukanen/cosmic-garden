//! Clone things (not clowns though)!

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, identity::{IdentityMut, IdentityQuery, MachineIdentity, uniq::Uuid}, item::{container::Storage, ownership::{ItemSource, OwnedMut}}, roomloc_or_bust, show_help_if_needed, tell_user, thread::add_item_to_lnf, traits::Reflector, validate_access};

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
            let i_id = item.id().to_string();
            let n_id = new.id().to_string();
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
                } else {
                    tell_user!(ctx.writer, "You clown '{}' as '{}'.\n", i_id, n_id);
                }
            }
        } else {
            if let Some(e_arc) = {
                let r = loc.read().await;
                if let Some(e_arc) = r.entities.get(&ctx.args.as_m_id()) {
                    e_arc.clone().into()
                } else { None }
            } {
                let o_id = e_arc.read().await.id().to_string();
                let mut e = e_arc.read().await.shallow_clone();
                e.set_id(&e.id().re_uuid(), true).ok();
                let e_id = e.id().to_string();
                let e_arc = Arc::new(RwLock::new(e));
                ctx.world.write().await.entities.insert(e_id.as_m_id(), Arc::downgrade(&e_arc));
                loc.write().await.entities.insert(e_id.as_m_id(), e_arc);
                tell_user!(ctx.writer, "Clowning entity '{}' as '{}'…\n", o_id, e_id);
            } else {
                err_tell_user!(ctx.writer, "No '{}' found…\n", ctx.args);
            }
        }
    }
}

#[cfg(test)]
mod cmd_clone_tests {
    use std::io::Cursor;

    use crate::{cmd::{clone::CloneCommand, inventory::InventoryCommand, look::LookCommand}, r#const::SMALL_ITEM, ctx, get_operational_mock_librarian, get_operational_mock_life, identity::{IdentityQuery, uniq::Uuid}, item::{Item, container::Storage, ownership::Owner, weapon::{WeaponSize, WeaponSpec}}, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

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
        let real_id = item.id().to_string();
        p.write().await.inventory.try_insert(item).expect("Seriously? No space for a sm0l knife?");
        let state = ctx!(sup true, state, CloneCommand, "", s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Player { event_host: false, builder: true };
        let state = ctx!(sup true, state, CloneCommand, "", s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(sup true, state, CloneCommand, real_id.as_str(), s,c,w,p);
        let _ = ctx!(sup true, state, InventoryCommand, "", s,c,w,p,|out:&str| out.split("dinged").collect::<Vec<&str>>().len() >= 3 );
    }

    #[tokio::test]
    async fn cmd_clone_entity() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let _ = get_operational_mock_librarian!(c,w);
        let _ = get_operational_mock_life!(c,w);
        stabilize_threads!();
        let c = c.out;
        let state = ctx!(sup true, state, CloneCommand, "", s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Player { event_host: false, builder: true };
        let state = ctx!(sup true, state, CloneCommand, "", s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        p.write().await.config.show_id = true;
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(50);
        let state = ctx!(sup true, state, LookCommand, "", s,c,w,p,|out:&str| out.contains("A goblin"));
        // the gobbo's ID...
        let g= {
            if let Some(r_arc) = p.read().await.location.upgrade() {
                let lock = r_arc.read().await;
                let (_, ent_arc) = lock.entities.iter().nth(0).unwrap();
                ent_arc.read().await.id().to_string()
            } else { panic!("Where did the room go?!") }
        };
        let state = ctx!(sup true, state, CloneCommand, g.as_str(), s,c,w,p,|out:&str| out.contains("ning entity"));
        let _ = ctx!(sup true, state, LookCommand, "", s,c,w,p,|out:&str| out.split("goblin").collect::<Vec<&str>>().len() >= 3 );
    }
}
