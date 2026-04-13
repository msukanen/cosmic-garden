//! Command aliasing
//! and some central clearing house to act as an "interface" for some commands beyond Access::Player.

use std::{collections::HashMap};
use lazy_static::lazy_static;
use crate::io::DATA_PATH;

lazy_static! {
    /// Command aliasing lives here…
    pub(crate) static ref CMD_ALIASES: HashMap<String, String> = {
        let contents = std::fs::read_to_string(&format!("{}/cmd_alias.json", *DATA_PATH)).unwrap_or_default();
        serde_json::from_str(&contents).unwrap_or_default()
    };
}

use async_trait::async_trait;
use crate::{cmd::{Command, CommandCtx}, io::ClientState, player::ActivityType, player_or_bust, tell_user};
pub struct AbortCommand;

/// Abort currently ongoing editing. Modifications done will *not* carry over.
#[async_trait]
impl Command for AbortCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        let plr = player_or_bust!(ctx);
        plr.write().await.activity_type = ActivityType::Playing;
        ctx.state = ClientState::Playing { player: plr.clone() };

        // Purge all the buffers.
        plr.write().await.iedit_buffer = None;
        plr.write().await.iedit_buffer = None;
        plr.write().await.iedit_buffer = None;
        
        if !ctx.args.starts_with('q') {
            tell_user!(ctx.writer, "Edits erased. Resuming normal life…\n");
        }
    }
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
