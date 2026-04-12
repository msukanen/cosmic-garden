//! Item examiner, 'iex'.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, item::{Item, Itemized, container::{Storage, specs::StorageSpace}}, string::Describable, tell_user, validate_access};

pub struct IexCommand;

#[async_trait]
impl Command for IexCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, builder);
        let p_id = {
            let p = plr.read().await;
            let p_id = p.id().to_string();
            p_id
        };
        let Some(item) = &plr.read().await.iedit_buffer else {
            log::error!("Builder '{p_id}'.iedit_buffer evaporated mid-edit!");
            tell_user!(ctx.writer, "You could've sworn there was an item in works…\n");
            return ;
        };

        tell_user!(ctx.writer, "<c cyan>--- Item Examination Gantry ---</c>\n");
        tell_user!(ctx.writer, "{:>10}: <c white>{}</c>\n", "ID", item.id());
        tell_user!(ctx.writer, "{:>10}: <c white>{}</c>\n", "Title", item.title());
        tell_user!(ctx.writer, "{:>10}: <c green>{:?}</c>\n", "Type", item); // Show the enum guts (Primordial, etc.)
        if let Item::Primordial(v) = item {
        tell_user!(ctx.writer, "{:>10}: <c green>{}</c>\n", "Potential", v.potential());
        }
            
        // Access the storage interface if applicable
        tell_user!(ctx.writer, "{:>10}: <c yellow>{}</c>\n", "Size", item.size());
        let cap: Option<StorageSpace> = match item {
            Item::Container(c) => Some(c.max_space()),
            Item::Primordial(p) => Some(p.max_space),
            _ => None
        };
        if let Some(cap) = cap {
        tell_user!(ctx.writer, "{:>10}: <c yellow>{}</c>\n", "Max space", cap);
        }

        // Nutrition spex
        let nutri = match item {
            Item::Consumable(v) => Some(
                (   v.nutrition.clone().into(),
                    v.uses,
                    v.affect_ticks
                )),
            Item::Primordial(v) => Some(
                (   v.nutrition.clone(),
                    v.uses,
                    v.affect_ticks
                )),
            _ => None
        };
        match nutri {
            None => (),
            Some((a,b,c)) => tell_user!(ctx.writer,
                "{:>10}: (type: {}, uses: {}, ticks: {})", "Nutrition",
                    match a {
                        None => "<n/a>".into(),
                        Some(v) => v.to_string(),
                    },
                    match b {None => "∞".into(), Some(v) => v.to_string()},
                    match c {None => "∞".into(), Some(v) => v.to_string()},
            ),
        }
        // 
        // Description    
        tell_user!(ctx.writer, "<c gray>Description:</c>\n{}\n", item.desc());
        tell_user!(ctx.writer, "<c cyan>-------------------------------</c>\n");
    }
}
