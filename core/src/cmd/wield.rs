//! Wield (or try to wield) something as a weapon…

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, item::container::Storage, player_or_bust, roomloc_or_bust, tell_user, thread::add_item_to_lnf};

pub struct WieldCommand;

#[async_trait]
impl Command for WieldCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        let loc = roomloc_or_bust!(plr);

        let what = ctx.args;
        {   let p = plr.read().await;
            if let Some(w) = &p.equipped_weapon {
                if w.eq(what) {
                    tell_user!(ctx.writer, "It's in your hands already, by the way, if you didn't notice…\n");
                    return ;
                }
            }
        }
        // see if the player has the item somewhere…
        {   let mut p = plr.write().await;
            if let Some(w) = p.inventory.take_by_name(what) {
                let Some(eq) = p.equipped_weapon.take() else {
                    unreachable!("Already checked to exist.");
                };
                let eq_title = eq.title().to_string();
                let mut fell = false; let mut vanished = false;
                if let Err(e) = p.inventory.try_insert(eq) {
                    tell_user!(ctx.writer, "You try to put '{}' among your belongings…\n  …alas, no space… and it falls on the ground while you're at it.\n", eq_title);
                    if let Err(e) = loc.write().await.contents.try_insert(e.extract_item()) {
                        tell_user!(ctx.writer, "  … but to your great surprise, someone or something nabbed it before it even touched the ground!\n");
                        add_item_to_lnf(e.extract_item()).await;
                        vanished = true;
                    }
                    fell = true;
                }
                let w_title = w.title().to_string();
                p.equipped_weapon = Some(w);
                tell_user!(ctx.writer, "You equip {}.{}\n", w_title,
                    if vanished {
                        format!(" Wonder where {eq_title} ended up at…?")
                    } else if fell {
                        format!(" {eq_title} lies on the ground but you don't have any other place to put it.")
                    } else {"".into()}
                    );
                return ;
            }
        }

        tell_user!(ctx.writer, "As much as you look for '{}', you don't seem to possess any such…\n", what);
    }
}

#[cfg(test)]
mod cmd_wield_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{get::GetCommand, look::LookCommand, shutdown::ShutdownCommand}, ctx, get_operational_mock_janitor, get_operational_mock_librarian, get_operational_mock_life, io::ClientState, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn wield_knife_ok() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,p,d) = get_operational_mock_world().await;
        let jt = get_operational_mock_janitor!(c,w,d.0);
        let lt = get_operational_mock_librarian!(c,w);
        let gt = get_operational_mock_life!(c,w);
        let c = c.out;// we don't need the c.recv part anymore here…
        tokio::time::sleep(Duration::from_secs(2)).await;// let the threads stabilize…
        log::debug!("Sending…");
        c.life.send(SystemSignal::Spawn { what: SpawnType::Item { id: "knife".into() }, room_id: "r-1".into() }).ok();
        let state = ClientState::Playing { player: p.clone() };
        tokio::time::sleep(Duration::from_secs(1)).await;// let the threads stabilize…
        let state = ctx!(state, LookCommand, "", s,c,w,p);
        let state = ctx!(state, GetCommand, "knife", s,c,w,p,|out:&str| out.contains("nab"));
        log::debug!("Got the knife!");
        p.write().await.access = Access::Admin;
        let state = ctx!(state, ShutdownCommand, "", s,c,w,p);
        _ = d.1.await;
    }
}