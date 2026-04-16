//! User/login stuff.

use std::fmt::Display;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{error::CgError, identity::IdError, io::user_save_fp, password::{PasswordError, validate_passwd}, string::Slugger};

/// Generic user info.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInfo {
    pub(super) id: String,
    /// User's players by file-system ID and printable character name.
    #[serde(default)]
    pub(super) players: Vec<(String, String)>,
    /// Argon2 hash.
    pub(super) argon2: String,
}

impl UserInfo {
    /// Load user info chunk.
    pub async fn load(id: &str, pwd: &str) -> Result<UserInfo, CgError> {
        let info: UserInfo = serde_json::from_str(
            &fs::read_to_string(user_save_fp(&id.as_id()?)).await?
        )?;
        if info.verify_passwd(pwd) {
            return Ok(info);
        }
        Err(CgError::from(IdError::PasswordMismatch))
    }

    /// Save the user info chunk.
    pub async fn save(&self) -> Result<(), CgError> {
        fs::write(user_save_fp(&self.id), serde_json::to_string_pretty(self)?).await?;
        Ok(())
    }

    /// Set password.
    /// 
    /// # Arguments
    /// - `plaintext_password`— new password.
    /// 
    /// # Returns
    /// Most likely `Ok`…
    pub async fn set_passwd<S>(&mut self, plaintext_passwd: S) -> Result<(), PasswordError>
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
    pub async fn argonize_passwd<S>(plaintext_passwd: S) -> Result<String, PasswordError>
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

    /// Create a brand new [UserInfo]
    pub async fn new(name: &str, pwd: &str) -> Result<Self, CgError> {
        let info = Self {
            argon2: UserInfo::argonize_passwd(pwd).await?,
            id: name.into(),
            players: vec![]
        };
        info.save().await?;
        Ok(info)
    }
}
