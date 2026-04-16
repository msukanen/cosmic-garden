//! Command aliasing
//! and some central clearing house to act as an "interface" for some commands beyond Access::Player.

use std::{collections::HashMap};
use lazy_static::lazy_static;

lazy_static! {
    /// Command aliasing lives here…
    pub(crate) static ref CMD_ALIASES: HashMap<String, String> = {
        let contents = std::fs::read_to_string(cmd_alias_fp()).unwrap_or_default();
        serde_json::from_str(&contents).unwrap_or_default()
    };
}

use async_trait::async_trait;
use crate::{cmd::{Command, CommandCtx}, io::{ClientState, cmd_alias_fp}, player::ActivityType, player_or_bust, tell_user};

pub struct BufferNuke;
/// Core of all 'abort' commands.
/// 
/// Abort currently ongoing editing. Any and all edits will get irredeemably *purged*.
/// To avoid unintended erasure of edits, modify the editor-specific 'abort' commands
/// to take such precautions, if wanted.
/// 
#[async_trait]
impl Command for BufferNuke {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        plr.write().await.activity_type = ActivityType::Playing;
        ctx.state = ClientState::Playing { player: plr.clone() };
        // Purge all the buffers.
        plr.write().await.purge_buffers();
        
        // `q`uiet flag?
        if !ctx.args.starts_with('q') {
            tell_user!(ctx.writer, "Edits erased. Resuming normal life…\n");
        }
    }
}

#[macro_export]
macro_rules! cmd_xedit_title {
    ($ctx:ident, $ed:ident) => {{
        let plr = crate::validate_access!($ctx, builder);
        crate::show_help_if_needed!($ctx, "edit-title");

        let mut lock = plr.write().await;
        paste::paste! {
        let Some(ref mut ed) = lock.[<$ed _buffer>] else {
            crate::tell_user!($ctx.writer, "Something weird in the neighborhood…\n");
            return;
        };
        }
        ed.set_title($ctx.args);
        crate::tell_user!($ctx.writer, "Shadow buffer title set to: {}\n", $ctx.args);
        $ctx.state.set_dirty(true);
    }};
}

#[macro_export]
macro_rules! cmd_xedit_desc {
    ($iam:expr, $ctx:ident, $ed:ident, $ed_v:literal) => {{
        let plr = crate::validate_access!($ctx, builder);
        let res = crate::util::ed::edit_text($ctx.writer, $ctx.args, crate::access_ed_entry!(plr, $ed).desc()).await;
        let verbose = match res {
            Ok(crate::util::ed::EdResult::ContentReady { text, verbose, .. }) => {
                let Some(ref mut b) = plr.write().await.iedit_buffer else {
                    log::error!("Whatever happened to Iedit buffer here...?");
                    return ;
                };
                b.set_desc(&text);
                $ctx.state.set_dirty(true);
                verbose
            },
            Ok(crate::util::ed::EdResult::NoChanges(true)) => true,
            Ok(crate::util::ed::EdResult::HelpRequested) => {
                crate::show_help!($ctx, "edit-desc");
            },
            _ => false
        };
        
        if verbose {// re-run argless to pretty-print current description.
            $iam.exec({$ctx.args = ""; $ctx}).await;
        }

        if $ctx.args.starts_with('?') {
            crate::show_help!($ctx, "edit-desc");
        }
    }};
}
#[cfg(test)]
mod cmd_alias_tests {
    use std::env;

    use crate::{DATA, cmd::cmd_alias::CMD_ALIASES};

    #[test]
    fn cmd_alias_reads() {
        let _ = DATA.set(env::var("COSMIC_GARDEN_DATA").unwrap());
        let _ = (*CMD_ALIASES).clone();
        assert_eq!("inventory".to_string(), *CMD_ALIASES.get("inv").unwrap());
    }
}
