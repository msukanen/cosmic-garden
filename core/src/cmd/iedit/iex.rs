//! Item examiner, 'iex'.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, identity::IdentityQuery, item::{Item, Itemized, container::{Storage, specs::StorageSpace}, ownership::Owned}, string::Describable, tell_user, tell_userln, validate_access};

pub struct IexCommand;

static IEX_UNSET: &'static str = "<c gray><unset></c>";

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

        tell_userln!(ctx.writer, "<c cyan>--- Item Examination Gantry ---</c>");
        // .id
        tell_userln!(ctx.writer, "{:>10}: <c white>{}</c>", "ID", item.id());
        // .title
        tell_userln!(ctx.writer, "{:>10}: <c white>{}</c>", "Title", item.title());
        // {:?}
        tell_userln!(ctx.writer, "{:>10}: <c green>{:?}</c>", "Type", item); // Show the enum guts (Primordial, etc.)

        // .potential
        if let Item::Primordial(v) = item {
        tell_userln!(ctx.writer, "{:>10}: <c green>{}</c>", "Potential", v.potential());
        }

        // .owner
        tell_userln!(ctx.writer, "{:>10}: <c white>{:?}</c>", "Owner", item.owner());
        
        // Access the storage interface if applicable
        tell_userln!(ctx.writer, "{:>10}: <c yellow>{}</c>", "Size", item.size());
        let cap: Option<StorageSpace> = match item {
            Item::Container(c) => Some(c.max_space()),
            Item::Primordial(p) => Some(p.max_space),
            _ => None
        };
        if let Some(cap) = cap {
        tell_userln!(ctx.writer, "{:>10}: <c yellow>{}</c>", "Max space", cap);
        }

        // .nutrition
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
            Some((a,b,c)) => tell_userln!(ctx.writer,
                "{:>10}: (type: {}, uses: {}, ticks: {})", "Nutrition",
                    match a {
                        None => "<n/a>".into(),
                        Some(v) => v.to_string(),
                    },
                    match b {None => "∞".into(), Some(v) => v.to_string()},
                    match c {None => "∞".into(), Some(v) => v.to_string()},
            ),_=>()
        }
        
        // .matter_state
        let matter_state = match item {
            Item::Consumable(v) => v.matter_state.into(),
            Item::Primordial(v) => v.matter_state.clone(),
            _ => None
        };
        match matter_state {
            Some(a) => tell_userln!(ctx.writer, "{:>10}: <c white>{}</c>", "Mat.State", a),_=>()
        }

        // .base_dmg
        let base_dmg = match item {
            Item::Primordial(v) => v.base_dmg.clone(),
            Item::Weapon(w) => w.base_dmg.into(),
            _ => None
        };
        match base_dmg {
            Some(a) => tell_userln!(ctx.writer, "{:>10}: <c white>{}</c>", "Base dmg", a),_=>()
        }
        
        // 
        // Description    
        tell_user!(ctx.writer, "<c gray>Description:</c>\n{}\n", item.desc());
        tell_user!(ctx.writer, "<c cyan>-------------------------------</c>\n");
    }
}
