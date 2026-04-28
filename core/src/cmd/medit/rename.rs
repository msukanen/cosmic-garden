//! Rename/Re-ID a mob.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_tell_user, identity::{IdentityMut, IdentityQuery}, show_help, show_help_if_needed, tell_user, util::access::Accessor, validate_access, validate_editor_mode};

pub struct RenameCommand;

#[async_trait]
impl Command for RenameCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        validate_editor_mode!(ctx, "MEdit");
        show_help_if_needed!(ctx, "rename");

        let (what, args) = ctx.args.split_once(' ').unwrap_or((ctx.args, ""));
        let (mut w, access) = {
            let w = plr.write().await;
            let access = w.access.clone();
            (w, access)
        };
        if let Some(ent) = &mut w.medit_buffer {
            match what {
                "id" => {
                    if args.is_empty() {
                        drop(w);
                        show_help!(ctx, "rename");
                    }
                    if !access.is_admin() {
                        drop(w);
                        err_tell_user!(ctx.writer, "<c red>[ERR]</c> Re-ID requires admin privileges.\n");
                    }
                    let old_id = ent.id().to_string();
                    if let Ok(_) = ent.set_id(args) {
                        let ent_id = ent.id().to_string();
                        ctx.state.set_dirty(true);
                        drop(w);
                        tell_user!(ctx.writer, "Entity '{}' re-ID'd as '{}'.\n", old_id, ent_id);
                        return ;
                    } else {
                        drop(w);
                        err_tell_user!(ctx.writer, "Can't call them '{}', sorry. Try something else…\n", args);
                    }
                },
                // normal rename:
                _ => {
                    let old_title = ent.title().to_string();
                    ent.set_title(ctx.args);
                    ctx.state.set_dirty(true);
                    drop(w);
                    tell_user!(ctx.writer, "Entity '{}' renamed as '{}'.\n", old_title, ctx.args);
                }
            }
        } else {
            log::error!("Builder medit_buffer evaporated while in MEdit mode?!");
            tell_user!(ctx.writer, "You could've sworn you were editing something, but…\n");
        }
    }
}

#[cfg(test)]
mod medit_rename_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::medit::{MeditCommand, rename::RenameCommand}, ctx, get_operational_mock_librarian, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn rename_normal() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let _ = get_operational_mock_librarian!(c,w);
        tokio::time::sleep(Duration::from_secs(1)).await;// let the thread(s) stabilize…
        let c = c.out;
        let state = ctx!(state, MeditCommand, "goblin",s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(state, MeditCommand, "goblin",s,c,w,p);
        let state = ctx!(state, RenameCommand, "",s,c,w,p);
        let state = ctx!(state, RenameCommand, "Hoblin",s,c,w,p,|out:&str| out.contains("Hoblin"));
        let state = ctx!(state, RenameCommand, "ixd hoblin",s,c,w,p,|out:&str| out.contains("rename <name>"));
        let state = ctx!(state, RenameCommand, "id hoblin",s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Admin;
        let _ = ctx!(state, RenameCommand, "id hoblin",s,c,w,p,|out:&str| out.contains("re-ID"));
    }
}
