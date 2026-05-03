//! Devolve a normally immutable potential item back to "primordial soup"-state.

use async_trait::async_trait;

use crate::{cmd::{Command, CommandCtx}, err_iedit_buffer_inaccessible, identity::IdentityQuery, item::{Item, container::storage::Storage}, roomloc_or_bust, tell_user, thread::janitor::add_item_to_lnf, validate_access};

pub struct DevolveCommand;

#[async_trait]
impl Command for DevolveCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = validate_access!(ctx, true_builder);
        let p_loc = roomloc_or_bust!(plr);
        let p_id = {
            let p = plr.read().await;
            let p_id = p.id().to_string();

            p_id
        };
        // contents safety net
        let mut p = plr.write().await;
        let Some(ed) = p.iedit_buffer.as_mut() else {
            err_iedit_buffer_inaccessible!(ctx,p,p_id);
        };
        if matches!(ed, Item::Primordial(_)) {
            tell_user!(ctx.writer, "It is already primordial soup. Can't reduce it any further…\n");
            return;
        }
        let ed_name = ed.title().to_string();
        if let Item::Container(v) = ed {
            if let Some(items) = v.eject_all() {
                drop(p);// deadlock prevention

                tell_user!(ctx.writer, "Suddenly '{}' spills the beans all over the place!\n", ed_name);
                let mut r = p_loc.write().await;
                let mut count_lost: usize = 0;
                for item in items {
                    // we try trust the poor Room to hold on to all of this junk...
                    if let Err(e) = r.try_insert(item) {
                        count_lost += 1;
                        add_item_to_lnf(e.extract_item()).await;
                    }
                }
                if count_lost > 0 {
                    tell_user!(ctx.writer, "Well bugger, you saw {} item{} to just evaporate?!\n",
                        match count_lost {
                            1 => "an",
                            2 => "two",
                            3..=5 => "a handful",
                            6..=10 => "many",
                            _ => "lots of"
                        }, if count_lost==1{""} else {"s"}
                    );
                }
            }
        }
        // reacquire the 'ed'
        let mut p = plr.write().await;
        let Some(ed) = p.iedit_buffer.as_mut() else {
            err_iedit_buffer_inaccessible!(ctx,p,p_id);
        };
        ed.devolve();
        tell_user!(ctx.writer, "You reduce '{}' into shimmering sludge… You can either:\n * keep working on it.\n * <c yellow>weave</c> it as-is (eww!)\n", ed_name);
    }
}
