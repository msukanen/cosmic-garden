//! Item examiner, 'iex'.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, item::Itemized, string::Describable, tell_user, validate_access};

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
        tell_user!(ctx.writer, "ID:       <c white>{}</c>\n", item.id());
        tell_user!(ctx.writer, "Title:    <c white>{}</c>\n", item.title());
        tell_user!(ctx.writer, "Type:     <c green>{:?}</c>\n", item); // Show the Enum Variant (Primordial, Vessel, etc)
            
        // Access the storage interface if applicable
        tell_user!(ctx.writer, "Size:     <c yellow>{}</c>\n", item.size());
            
        tell_user!(ctx.writer, "<c gray>Description:</c>\n{}\n", item.desc());
        tell_user!(ctx.writer, "<c cyan>-------------------------------</c>\n");
    }
}
