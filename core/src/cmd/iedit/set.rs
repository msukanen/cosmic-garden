//! Set some detail about an item in the IEDit.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_iedit_buffer_inaccessible, identity::{IdentityMut, IdentityQuery}, item::{Item, ItemizedMut, container::{StorageMut, specs::StorageSpace}, primordial::PotentialItemType}, tell_user, validate_access};

pub struct SetCommand;

macro_rules! no_can_do {
    ($ctx:ident, $what:expr) => {
        {
            tell_user!($ctx.writer, "Item's {} is immutable, sorry.\n", $what);
            return;
        }
    };
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

            "size" => {
                if let Ok(sz) = value.parse::<StorageSpace>() {
                    if !ed.set_size(sz) {
                        tell_user!(ctx.writer, "That item's size is immutable, sorry…\n");
                        return;
                    }
                    tell_user!(ctx.writer, "Size set to: {}\n", sz);
                }
            },

            "max_space"|
            "max" => {
                if let Ok(sz) = value.parse::<StorageSpace>() {
                    if !ed.set_max_space(sz) {
                        tell_user!(ctx.writer, "Ugh, too much stuff in there…\nMight consider <c yellow>'weave'</c> and put them things elsewhere first.\n");
                        return;
                    }
                    tell_user!(ctx.writer, "Max space set to: {}\n", sz);
                }
            },

            "potential"|
            "pot" => {
                if !matches!(ed, Item::Primordial(_)) {
                    no_can_do!(ctx, "potential");
                }
                let err = PotentialItemType::from(value);
                if err.is_err() {
                    tell_user!(ctx.writer, "That doesn't work, the variants are: {}\n", err.err().unwrap());
                    return;
                };
                let pot = err.ok().unwrap();
                ed.set_potential(pot.clone());
                tell_user!(ctx.writer, "Item potential set as '{}'\n", pot);
            },

            _ => tell_user!(ctx.writer, "No such field to alter, and I can't just whip up new fields out of nothing…\n")
        }
    }
}
