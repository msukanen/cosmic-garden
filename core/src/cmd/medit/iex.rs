//! IEx for MEdit.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, cmd_alias::BufferNuke}, combat::Combatant, err_tell_user, identity::IdentityQuery, mob::{faction::Factioned, traits::Mob}, string::Describable, tell_user, validate_access, validate_editor_mode};

pub struct IexCommand;

#[async_trait]
impl Command for IexCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        validate_editor_mode!(ctx, "MEdit");
        let p = plr.read().await;
        let ent = {
            let Some(ent) = &p.medit_buffer else {
                drop(p);
                log::error!("Builder medit_buffer evaporated?!");
                BufferNuke.exec({ctx.args = "";ctx}).await;
                err_tell_user!(ctx.writer, "There's nothing in your edit buffer? Huh…\n");
            };
            ent.clone()
        };

        let mut out = vec![];
        out.push(format!("{:>10}: <c gray>{}</c>", "ID", ent.id()));
        out.push(format!("{:>10}: {}", "Name", ent.title()));
        out.push("".into());
        out.push(format!("{:>10}: {:>3}/{:<3}     {:>2}: {:>3}/{:<3}     {:>2}: {:>3}/{:<3}",
                         "HP", ent.hp().current(), ent.hp().max(),
                         "MP", ent.mp().current(), ent.mp().max(),
                         "SN", ent.sn().current(), ent.sn().max(),
                        ));
        out.push(format!("{:>10}: {:>3}/{:<3}    {:>3}: {:>3}/{:<3}   {:>4}: {:>3}/{:<3}",
                         "BRN", ent.brn().current(), ent.brn().max(),
                         "NIM", ent.nim().current(), ent.nim().max(),
                         "STRN", ent.str().current(), ent.str().max(),
                        ));
        out.push(format!("{:>10}: {:>3}/{:<3}", "SAN", ent.san().current(), ent.san().max()));
        out.push("".into());
        out.push(format!("{:>10}: {:<10}     Max.WpnSize: {}", "Stature", ent.size(), ent.max_weapon_size()));
        out.push(format!("{:>10}: {:<}", "Faction", ent.faction()));
        out.push(format!("{:>10}: {:<}", "Equipped", 
            if let Some(wpn) = &ent.equipped_weapon {
                wpn.title().to_string()
            } else { "None".into() }
        ));
        out.push(format!("<c gray>Description:</c>\n{}", ent.desc()));
        tell_user!(ctx.writer, "{}\n", out.join("\n"));
    }
}

#[cfg(test)]
mod medit_iex_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::{look::LookCommand, medit::{MeditCommand, iex::IexCommand, rename::RenameCommand, weave::WeaveCommand}}, ctx, get_operational_mock_librarian, get_operational_mock_life, io::ClientState, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn iex_test() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,p,_) = get_operational_mock_world().await;
        // we don't need janitor running as we're not persisting anything onto disk here …
        let _ = get_operational_mock_life!(c,w);
        let _ = get_operational_mock_librarian!(c,w);
        let c = c.out;
        tokio::time::sleep(Duration::from_secs(1)).await;// let the threads stabilize…
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".to_string() }, room_id: "r-1".into() }).ok();
        tokio::time::sleep(Duration::from_millis(75)).await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(sup true, state, MeditCommand, "", s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        p.write().await.config.show_id = true;
        // we know r-1 exists…
        let r1 = w.read().await.rooms.get("r-1").unwrap().clone();
        let id = {
            let r1l = r1.read().await;
            let Some(e) = r1l.entities.keys().find(|x| x.starts_with("goblin")) else {
                panic!("Where'd the lil goblin go?!");
            };
            e.clone()
        };
        let state = ctx!(sup true, state, MeditCommand, &format!("{id}"), s,c,w,p,|out:&str| out.contains("MEdit invoked"));
        let state = ctx!(sup true, state, RenameCommand, "Morg-Gluglug", s,c,w,p,|out:&str| out.contains("renamed"));
        let state = ctx!(sup true, state, LookCommand,"",s,c,w,p,|out:&str| out.contains("goblin"));
        let state = ctx!(sup true, state, WeaveCommand, "",s,c,w,p);
        let state = ctx!(sup true, state, LookCommand,"",s,c,w,p,|out:&str| out.contains("Morg-Glug"));
        let state = ctx!(sup true, state, MeditCommand, &format!("{id}"), s,c,w,p,|out:&str| out.contains("MEdit invoked"));
        let _ = ctx!(state, IexCommand, "", s,c,w,p,|out:&str| out.contains("Faction"));
    }
}
