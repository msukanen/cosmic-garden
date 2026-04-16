//! All sorts of prompt related stuff.
use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum PromptType {
    Login,
    Password1, PasswordV, PasswordVFail,
    PlayerChooser0, PlayerChooserM, NamingViolation, PlayerChooserOOB,
    Playing,
    AFK,
    Custom(String),
    SystemError,
}

impl Display for PromptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::AFK => "<AFK> ",
            Self::Custom(v) => v.as_str(),
            Self::Login => "Login",
            Self::NamingViolation => "There was an error with that name, give a new one",
            Self::Password1 => "Password",
            Self::PasswordV => "Re-type same password",
            Self::PasswordVFail => "Word mismatch, re-type them…",
            Self::PlayerChooser0 => "Enter name for your first character",
            Self::PlayerChooserM => "Enter name for new character or pick an existing one by index",
            Self::PlayerChooserOOB => "Try a smaller index value…",
            Self::SystemError => "Something went awfully awry. Contact an ADMIN ASAP!",

            Self::Playing => unimplemented!("Active Player instances have their custom prompts, dealt with elsewhere"),
        })
    }
}

#[macro_export]
macro_rules! tell_user {
    ($w:expr, $t:expr) => {
        tokio::io::AsyncWriteExt::write_all($w, crate::string::styling::format_color($t).as_bytes()).await.unwrap()
    };

    ($w:expr, $fmt:literal, $($arg:tt)*) => {{
        let msg = format!($fmt, $($arg)*);
        crate::tell_user!($w, &msg);
    }}
}

#[macro_export]
macro_rules! tell_userln {
    ($w:expr, $t:expr) => {{
        crate::tell_user!($w, "{}\n", $t);
    }};

    ($w:expr, $fmt:literal, $($arg:tt)*) => {{
        let msg = format!($fmt, $($arg)*);
        crate::tell_userln!($w, &msg);
    }}
}

#[macro_export]
macro_rules! tell_user_unk {
    ($w:expr) => {
        crate::tell_user!($w, "Huh?\n")
    };
}

#[macro_export]
macro_rules! get_prompt {
    ($w:expr, $ptype:expr) => {
        { $w.read().await.fixed_prompts.get(&$ptype).cloned().unwrap_or_else(|| $ptype.to_string()) }
    };
}

#[macro_export]
macro_rules! reprompt_playing_user {
    ($writer:ident, $state:ident) => {{
        let prompt = match &$state {
            ClientState::Playing {player}|
            ClientState::Editing {player,..}=> {
                let r = player.read().await;
                r.prompt(&$state)
            }
            _=> None
        };
        tell_user!(&mut $writer, "{}", prompt.unwrap_or("".into()));
    }};
}
