//! Set some detail about an item in the IEDit.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::{IdentityMut, IdentityQuery}, item::{ItemizedMut, container::{StorageMut, specs::StorageSpace}}, tell_user, validate_access};

pub struct SetCommand;

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
 * max

For description, use 'desc' command instead.
"#);
            return;
        }

        let mut p = plr.write().await;
        let Some(ed) = p.iedit_buffer.as_mut() else {
            log::error!("Builder '{p_id}'.iedit_buffer evaporated mid-edit?!");
            drop(p);
            tell_user!(ctx.writer, "Uh-oh, editor buffer evaporated?!\n");
            return;
        };

        match field {
            "title" => {
                ed.set_title(value);
                tell_user!(ctx.writer, "Title set to: {}\n", value);
            },

            "size" => {
                if let Ok(sz) = value.parse::<StorageSpace>() {
                    if !ed.set_size(sz) {
                        tell_user!(ctx.writer, "That item's size is immutable, sorry…\n");
                        return;
                    }
                    tell_user!(ctx.writer, "Size set to: {}\n", sz);
                }
            },

            "max" => {
                if let Ok(sz) = value.parse::<StorageSpace>() {
                    if !ed.set_max_space(sz) {
                        tell_user!(ctx.writer, "Ugh, too much stuff in there…\nMight consider <c yellow>'weave'</c> and put them things elsewhere first.\n");
                        return;
                    }
                    tell_user!(ctx.writer, "Max space set to: {}\n", sz);
                }
            },

            _ => tell_user!(ctx.writer, "No such field to alter, and I can't just whip up new fields out of nothing…\n")
        }
    }
}
