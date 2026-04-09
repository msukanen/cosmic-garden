//! I/O related stuff lives here…

use std::{fmt::Display, fs, ops::Deref, path::{Path, PathBuf}, sync::{Arc, Weak}};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::{net::tcp::OwnedWriteHalf, sync::RwLock};

use crate::{edit::EditorMode, error::Error, get_prompt, identity::{IdError, IdentityMut, IdentityQuery}, password::{PasswordError, validate_passwd}, player::Player, string::{Slugger, prompt::PromptType}, tell_user, world::World};

/// ImmutablePath to appease lazy-init file system access…
pub(crate) struct ImmutablePath; impl ImmutablePath {
    pub fn set(path: impl Into<String>) {
        let path: String = path.into();
        DATA.set(path.clone()).expect(&format!("Cannot set DATA to '{path}'!"));
    }
}

/// Deref to appease lazy-init file system access…
impl Deref for ImmutablePath {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        DATA.get().unwrap_or_else(|| {
            panic!("DATA.get() fail. DATA_PATH var not set yet? Dev, go find out why not…");
        })
    }
}

pub(super) static DATA: OnceCell<String> = OnceCell::new();
pub(crate) static DATA_PATH: ImmutablePath = ImmutablePath;
lazy_static! {
    pub(crate) static ref SAVE_PATH: PathBuf = PathBuf::from(format!("{}/save", *DATA_PATH));
}

/// Various states of client existence…
#[derive(Debug, Clone)]
pub enum ClientState {
    EnteringLogin,
    EnteringPassword1 { name: String },
    EnteringPasswordV { name: String, pw1: String },
    ChoosingPlayer { info: UserInfo },
    Playing { player: Arc<RwLock<Player>> },
    Editing { player: Arc<RwLock<Player>>, mode: EditorMode },
    Logout,
}

impl PartialEq for ClientState {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::EnteringLogin => matches!(other, Self::EnteringLogin),
            Self::EnteringPassword1 { .. } => matches!(other, Self::EnteringPassword1 { .. }),
            Self::EnteringPasswordV { .. } => matches!(other, Self::EnteringPasswordV { .. }),
            Self::Playing { .. } => matches!(other, Self::Playing { .. }),
            Self::Logout => matches!(other, Self::Logout),
            Self::Editing { mode, .. } => {
                let mode1 = mode;
                match other {
                    Self::Editing { mode, .. } => *mode1 == *mode,
                    _ => false
                }
            },
            Self::ChoosingPlayer { .. } => matches!(other, Self::ChoosingPlayer { .. }),
        }
    }
}

/// Generic user info.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInfo {
    id: String,
    /// User's players by file-system ID and printable character name.
    #[serde(default)]
    players: Vec<(String, String)>,
    argon2: String,
}

impl UserInfo {
    async fn load(id: &str, pwd: &str) -> Result<UserInfo, Error> {
        let info: UserInfo = serde_json::from_str(
            &fs::read_to_string(&format!("{}/{}", SAVE_PATH.display(), id.as_id()?))?
        )?;
        if info.verify_passwd(pwd) {
            return Ok(info);
        }
        Err(Error::from(IdError::PasswordMismatch))
    }

    async fn save(&self) -> Result<(), Error> {
        fs::write(format!("{}/{}", SAVE_PATH.display(), self.id), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Set password.
    /// 
    /// # Arguments
    /// - `plaintext_password`— new password.
    /// 
    /// # Returns
    /// Most likely `Ok`…
    async fn set_passwd<S>(&mut self, plaintext_passwd: S) -> Result<(), PasswordError>
    where S: Display,
    {
        self.argon2 = Self::argonize_passwd(plaintext_passwd).await?;
        Ok(())
    }

    /// Argonize password.
    /// 
    /// # Arguments
    /// - `plaintext_password`— new password.
    /// 
    /// # Returns
    /// Most likely `Ok`…
    async fn argonize_passwd<S>(plaintext_passwd: S) -> Result<String, PasswordError>
    where S: Display,
    {
        validate_passwd(&plaintext_passwd.to_string()).await?;
        let salt = SaltString::generate(&mut OsRng);
        let pw_hash = Argon2::default()
            .hash_password(plaintext_passwd.to_string().as_bytes(), &salt)?
            .to_string();
        Ok(pw_hash)
    }

    /// Verify given password vs stored password.
    /// 
    /// # Arguments
    /// - `plaintext_passwd`— some passwordlike thing.
    fn verify_passwd<S>(&self, plaintext_passwd: S) -> bool
    where S: Display,
    {
        if self.argon2.is_empty() {
            return false;
        }

        // parse stored hash
        let parsed_hash = match PasswordHash::new(&self.argon2) {
            Ok(hash) => hash,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(plaintext_passwd.to_string().as_bytes(), &parsed_hash)
            .is_ok()
    }
}

impl ClientState {
    /// Is the player actually in game yet (or going away)?
    pub fn is_in_game(&self) -> bool {
        matches!(self, Self::Editing { .. }|Self::Playing { .. })
    }

    /// Big state handler…
    pub async fn handle(mut self, mut writer: &mut OwnedWriteHalf, world: Arc<RwLock<World>>, input: &str) -> Self {
        match self {
            Self::EnteringLogin => {
                let state = match input.as_id() {
                    Ok(name) => Self::EnteringPassword1 { name },
                    Err(e) => {
                        tell_user!(&mut writer, "Well... that login name fails: {}\n", e);
                        self
                    }
                };
                tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::Password1));
                state
            },

            Self::EnteringPassword1 { name } => {
                match UserInfo::load(&name, &input).await {
                    Ok(info) => {
                        if info.players.is_empty() {
                            tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::PlayerChooser0));
                        } else {
                            // list all the existing players for this login.
                            let mut out: String = String::new();
                            for (index, (_, name)) in info.players.iter().enumerate() {
                                out.push_str(&format!("    {:2}.  {}\n", index + 1, name));
                            }
                            tell_user!(&mut writer, "{}", out);
                            tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::PlayerChooserM));
                        }
                        Self::ChoosingPlayer { info }
                    },
                    // Brand new user:
                    Err(Error::Io(_)) => {
                        log::info!("Brand new user: {name}");
                        tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::PasswordV));
                        Self::EnteringPasswordV { name, pw1: input.to_string() }
                    },
                    Err(e) => {
                        log::warn!("Login failure for '{name}': {e:?}");
                        tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::Login));
                        Self::EnteringLogin
                    }
                }
            },

            Self::EnteringPasswordV { name, pw1 } => {
                if input == pw1 {
                    let info = UserInfo {
                        players: vec![],
                        argon2: match UserInfo::argonize_passwd(pw1).await {
                            Ok(argon2) => argon2,
                            Err(e) => {
                                log::warn!("Argonizing… {e:?}");
                                tell_user!(&mut writer, "{}\n{}", e, get_prompt!(world, PromptType::Password1));
                                return Self::EnteringPassword1 { name };
                            }
                        },
                        id: name,

                    };
                    if let Err(e) = info.save().await {
                        log::error!("FATAL: {e:?}");
                        tell_user!(&mut writer, "{}\n", get_prompt!(world, PromptType::SystemError));
                        return Self::Logout;
                    }
                    tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::PlayerChooser0));
                    return Self::ChoosingPlayer { info }
                }

                tell_user!(&mut writer, "{}\n", get_prompt!(world, PromptType::PasswordVFail));
                tell_user!(&mut writer, "{}\n", get_prompt!(world, PromptType::Password1));
                Self::EnteringPassword1 { name }
            },

            Self::ChoosingPlayer { ref mut info } => {
                if info.players.is_empty() {
                    // no players yet, make one
                    let mut p = Player::default();
                    if let Err(e) = p.set_id(input) {
                        log::warn!("IdError… {e:?}");
                        tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::NamingViolation));
                        return self;
                    }
                    p.owner_id = info.id.clone();
                    p.name = input.into();
                    if let Err(e) = p.save().await {
                        log::error!("FATAL: {e:?}");
                        tell_user!(&mut writer, "{}\n", get_prompt!(world, PromptType::SystemError));
                        return Self::Logout;
                    }
                    let p = Arc::new(RwLock::new(p));
                    let state = Self::Playing { player: p.clone() };
                    {
                        let lock = p.read().await;
                        tell_user!(&mut writer, "{}", lock.prompt(&state).unwrap_or_default());
                        info.players.push((lock.id().into(), lock.name.clone()));
                        if let Err(e) = info.save().await {
                            log::error!("FATAL: {e:?}");
                            tell_user!(&mut writer, "{}\n", get_prompt!(world, PromptType::SystemError));
                            return Self::Logout;
                        }
                    }
                    return state;
                }

                // did user give an index?
                if let Ok(num) = input.parse::<usize>() {
                    let num = num.saturating_sub(1);
                    if num >= info.players.len() {
                        // out of bounds, clearly…
                        tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::PlayerChooserOOB));
                        return self;
                    }

                    // printed indexes are 1+ to player; we need to wind the index back a bit - and treat user input of 0 as if they wrote 1 instead.
                    let (p_id, _) = &info.players[num];

                    if let Ok(player) = Player::load(&info.id, &p_id).await {
                        let state = Self::Playing { player: player.clone() };
                        tell_user!(&mut writer, "{}", player.read().await.prompt(&state).unwrap_or_default());
                        return state;
                    } else {
                        log::error!("UserInfo of user '{}' mismatch - Player file '{}' missing (or broken)!", info.id, p_id);
                        tell_user!(&mut writer, "A bit of misplacement error here… Do contact admin ASAP!\n");
                        return self;
                    }
                }

                if let Some((id, _)) = info.players.iter().find(|(id, name)| {
                    let check_vs = input.to_lowercase();
                    id == &check_vs || name.to_lowercase() == check_vs
                }) {
                    // existing character.
                    return match Player::load(&info.id, id).await {
                        Err(e) => {
                            log::error!("UserInfo of user '{}' mismatch - Player file '{}' missing (or broken)! {e:?}", info.id, id);
                            tell_user!(&mut writer, "A bit of misplacement error here… Do contact admin ASAP!");
                            self
                        }
                        Ok(player) => {
                            let state = Self::Playing { player: player.clone() };
                            tell_user!(&mut writer, "{}", player.read().await.prompt(&state).unwrap_or_default());
                            state
                        }
                    }
                }

                // brand new character, same procedure as further up:
                let mut p = Player::default();
                if let Err(e) = p.set_id(input) {
                    log::info!("Sloppy name writing… '{input}' does not function as an Id {e:?}");
                    tell_user!(&mut writer, "{}: ", get_prompt!(world, PromptType::NamingViolation));
                    return self;
                }
                p.owner_id = info.id.clone();
                p.name = input.into();
                if let Err(e) = p.save().await {
                    log::error!("FATAL: {e:?}");
                    tell_user!(&mut writer, "{}\n", get_prompt!(world, PromptType::SystemError));
                    return Self::Logout;
                }
                let p = Arc::new(RwLock::new(p));
                let state = Self::Playing { player: p.clone() };
                {
                    let lock = p.read().await;
                    tell_user!(&mut writer, "{}", lock.prompt(&state).unwrap_or_default());
                    info.players.push((lock.id().into(), lock.name.clone()));
                    if let Err(e) = info.save().await {
                        log::error!("FATAL: {e:?}");
                        tell_user!(&mut writer, "{}\n", get_prompt!(world, PromptType::SystemError));
                        return Self::Logout;
                    }
                }
                state
            },

            Self::Editing { ref player, .. } |
            Self::Playing { ref player }     => {
                tell_user!(&mut writer, "{}", player.read().await.prompt(&self).unwrap_or_default());
                self
            },

            Self::Logout => self,
        }
    }
}

/// Various broadcast types.
#[derive(Debug, Clone)]
pub enum Broadcast {
    Say {
        //room: Weak<RwLock<Room>>,
        message: String,
        //from: Weak<RwLock<>>
    }
}
