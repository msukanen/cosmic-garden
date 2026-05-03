//! Weave an entity
//! * into existence,
//! * as a blueprint, or
//! * inject edits into a live specimen.

use std::time::Duration;

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx, cmd_alias::BufferNuke, look::LookCommand}, err_tell_user, identity::{IdentityQuery, MachineIdentity, uniq::StrUuid}, roomloc_or_bust, show_help, tell_user, thread::{SystemSignal, librarian::shelve_entity_blueprint, signal::SpawnType}, validate_access, validate_editor_mode};

pub struct WeaveCommand;

#[async_trait]
impl Command for WeaveCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let loc = roomloc_or_bust!(plr);
        validate_editor_mode!(ctx, "MEdit");

        let (op, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        let Some(buf) = plr.write().await.medit_buffer.take() else {
            log::error!("Builder medit_buffer was empty?! Bug with MEdit?");
            err_tell_user!(ctx.writer, "You could've sworn you were editing specs of an entity, but no…?\n");
        };
        
        // "weave" w/o args
        if op.is_empty() {
            let w = ctx.world.read().await;
            if let Some(ent) = w.entities.get(&buf.id().as_m_id()) {
                if let Some(arc) = ent.upgrade() {
                    drop(w);
                    let mut lock = arc.write().await;
                    tell_user!(ctx.writer, "Modifications injected into '{}'… for the better or worse.\n", buf.id());
                    lock.copyback(buf);
                } else {
                    tell_user!(ctx.writer, "No can do … '{}' has left the mortal coil before weave.\n");
                    drop(w);
                }
                BufferNuke.exec({ctx.args = "q"; ctx}).await;
                return ;
            } else {
                drop(w);
                plr.write().await.medit_buffer = buf.into();
                tell_user!(ctx.writer, "No such entity exists yet. To spawn a BP into reality, <c yellow>weave persist spawn</c>.\n\n");
                show_help!(ctx, "q weave");
            }
        }
        
        match op {
            "persist" => {
                validate_access!(ctx, admin);
                let ent_stem = buf.id().show_uuid(false).to_string();
                shelve_entity_blueprint(&buf, &ctx.out).await;
                let mut spawn = false;
                if args == "spawn" {
                    spawn = true;
                    ctx.out.life.send(SystemSignal::Spawn {
                        what: SpawnType::Mob { id: ent_stem },
                        room: loc.read().await.id().into(),
                        reply: None
                    }).ok();
                }
                tell_user!(ctx.writer, "Entity blueprint persisted{}\n",
                    if spawn {
                        " and a live specimen based on it spawned."
                    } else {""});
                if !spawn { return; }
            }

            "spawn" => {
                ctx.out.life.send(SystemSignal::Spawn { what: SpawnType::Mob {
                    id: buf.id().show_uuid(false).to_string() },
                    room: loc.read().await.id().into(),
                    reply: None
                }).ok();
                tell_user!(ctx.writer, "{} is being spawned… hopefully.\n", buf.title());
            }

            _ => show_help!(ctx, "u weave")
        }

        BufferNuke.exec({ctx.args = "q"; ctx}).await;
        tokio::time::sleep(Duration::from_millis(75)).await;// leave life-thread plenty of time to comply (or to not to)…
        LookCommand.exec(ctx).await;
    }
}

#[cfg(test)]
mod medit_tests {
    use std::io::Cursor;

    use crate::{cmd::{look::LookCommand, medit::{MeditCommand, rename::RenameCommand, weave::WeaveCommand}}, ctx, edit::EditorMode, get_operational_mock_librarian, get_operational_mock_life, io::ClientState, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn weave_test() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        // we don't need janitor running as we're not persisting anything onto disk here …
        let _ = get_operational_mock_life!(c,w);
        let _ = get_operational_mock_librarian!(c,w);
        let c = c.out;
        stabilize_threads!(250);
        let state = ctx!(sup state, WeaveCommand, "", s,c,w,|out:&str| out.contains("Huh?"));
        let state = ctx!(sup state, MeditCommand, "", s,c,w,|out:&str| out.contains("Huh?"));
        assert!(p.read().await.medit_buffer.is_none());
        p.write().await.access = Access::Builder;
        let state = ctx!(sup state, WeaveCommand, "", s,c,w,|out:&str| out.contains("MEdit first"));
        let state = ctx!(sup state, MeditCommand, "", s,c,w,|out:&str| out.contains("Invokes"));
        let state = ctx!(sup state, MeditCommand, "goblin", s,c,w);
        assert!(matches!(state, ClientState::Editing { mode: EditorMode::Medit { .. },.. }));
        let state = ctx!(sup state, RenameCommand, "Hoblin! the mighty, and stuff!", s,c,w);
        let state = ctx!(sup state, RenameCommand, "id hoblin" ,s,c,w,|out:&str| out.contains("Re-ID"));
        p.write().await.access = Access::Admin;
        let state = ctx!(sup state, RenameCommand, "id hoblin" ,s,c,w,|out:&str| out.contains("re-ID'd"));
        let state = ctx!(sup state, WeaveCommand, "",s,c,w,|out:&str| out.contains("a no-op"));
        p.write().await.config.show_id = true;
        let state = ctx!(sup state, WeaveCommand, "persist spawn",s,c,w,|out:&str| out.contains("Hoblin!"));
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "hoblin".to_string() }, room: "r-1".into(), reply: None }).ok();
        stabilize_threads!(25);
        let _ = ctx!(state, LookCommand,"",s,c,w);
    }
}
