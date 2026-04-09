//! Everything has an identity, let's deal with that…
use std::{collections::HashSet, fmt::Display, fs::{self, read_to_string}, sync::Arc};

use lazy_static::lazy_static;
use tokio::sync::RwLock;

use crate::{io::DATA_PATH, string::{Uuid, slug::Slugger}};

pub const MAX_ID_LEN: usize = 255;

lazy_static! {
    /// Immutable [IdError::ReservedName] sources.
    static ref HARDCODED_RESERVED: HashSet<&'static str> = {
        let mut s = HashSet::new();
        // some OS-related things...
        for name in &[
            "con", "prn", "aux", "nul",
            "null", "dev", "root", "home",
            "usr", "etc", "var", "tmp",
        ] { s.insert(*name);}
        for i in 1..=9 {
            s.insert(Box::leak(format!("com{i}").into_boxed_str()));
            s.insert(Box::leak(format!("lpt{i}").into_boxed_str()));
        }
        // names, etc.
        for name in &[
            "admin", "sys", "system", "root",
            "world", "self", "me", "omfg",
        ] { s.insert(*name);}
        s
    };

    static ref CONFIG_RESERVED: Arc<RwLock<HashSet<String>>> = {
        let mut s = HashSet::new();
        if let Ok(buf) = fs::read_to_string(format!("{}/reserved.names", *DATA_PATH)) {
            let words = buf.split(';').map(|w| w.trim()).collect::<Vec<&str>>();
            for w in words {
                s.insert(w.into());
            }
        } else {
            log::trace!("No {}/reserved.names to process.", *DATA_PATH);
        }
        Arc::new(RwLock::new(s))
    };
}

/// Various ID related errors…
#[derive(Debug, PartialEq, Clone)]
pub enum IdError {
    /// Input was entirely non-alphanum (or empty).
    EmptyOrGarbage,
    /// Input too long (for e.g. file system).
    TooLong,
    /// Input contains forbidden/reserved (e.g. any of the hardcoded bootstrap) patterns.
    ReservedName(String),
    /// Password mismatch…
    PasswordMismatch,
}

impl Display for IdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyOrGarbage => write!(f, "Well, identity contains no readable alphanum characters. Might want to rething that…"),
            Self::ReservedName(n) => write!(f, "Sorry, but '{}' is already reserved by the system itself…", n),
            Self::TooLong => write!(f, "For file system sanity reasons the input was considered 'too long' (in excess of {} characters). Cut it down a bit, ok?", MAX_ID_LEN),
            Self::PasswordMismatch => write!(f, "Password mismatch…"),
        }
    }
}

impl std::error::Error for IdError {}

/// A trait for anything and everything with "identity".
pub trait IdentityQuery {
    /// Get the ID.
    fn id<'a>(&'a self) -> &'a str;
    /// Get the title/name.
    fn title<'a>(&'a self) -> &'a str;
}

/// A trait for anything and everything with mutable "identity".
pub trait IdentityMut {
    /// Get ref to raw ID.
    fn id_mut<'a>(&'a mut self) -> &'a mut String;
    /// Safely set ID, if possible.
    /// Given `value` will be slugged with [as_id()][crate::string::slug::Slugger::as_id].
    /// 
    /// # Args
    /// - `value` to use as ID.
    fn set_id(&mut self, value: &str) -> Result<(), IdError> {
        let pre_checked_id = value.as_id()?;
        if HARDCODED_RESERVED.contains(pre_checked_id.no_uuid().as_str()) {
            return Err(IdError::ReservedName(value.no_uuid()));
        }
        *self.id_mut() = pre_checked_id;
        Ok(())
    }

    /// Get ref to raw title/name.
    fn title_mut<'a>(&'a mut self) -> &'a mut String;
    /// Safely set title to `value`.
    // Convenience method really as titles in general are freeform, and thus lack validation.
    fn set_title(&mut self, value: &str) {
        *self.title_mut() = value.into()
    }
}

#[cfg(test)]
mod identity_tests {
    use cosmic_garden_pm::IdentityMut;

    use super::*;

    #[derive(IdentityMut)]
    struct Identifiable {
        id: String,
        #[identity(title)]
        nomnom: String,
    }

    #[test]
    fn identity_query_derive() {
        let i = Identifiable { id: "<an id>".into(), nomnom: "<a title>".into() };
        assert_eq!("<an id>", i.id());
        assert_eq!("<a title>", i.title());
    }

    #[test]
    fn identity_mut_derive() {
        let mut i = Identifiable { id: "<an id>".into(), nomnom: "<a title>".into() };
        *i.id_mut() = "<a mutant id>".into();
        *i.title_mut() = "<a mangy title>".into();
        assert_eq!("<a mutant id>", i.id());
        assert_eq!("<a mangy title>", i.title());
    }

    #[test]
    fn identity_mut_push() {
        let mut i = Identifiable { id: "<an id>".into(), nomnom: "<a title>".into() };
        i.title_mut().push_str(" is broken…");
        *i.title_mut() = i.title().replace("<a", "<t3h");
        assert_eq!("<t3h title> is broken…", i.title());
    }
}
