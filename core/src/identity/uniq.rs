//! UUID related things…

use lazy_static::lazy_static;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::{item::Item, util::escape_hatch::VILLAIN_ID};
use super::{IdError, IdentityMut, IdentityQuery};

lazy_static! {
    pub(crate) static ref UUID_RE: Regex = Regex::new(
        r"(?P<uuid>[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$"
    ).unwrap();
    pub(crate) static ref UUID_RE_DELIM: Regex = Regex::new(
        r"[-][0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
    ).unwrap();
}

pub trait UuidValidator {
    /// Get representation as file/hash ready `id`, if possible…
    fn as_id(&self) -> Result<String, IdError>;
    /// Check if string is valid as ID without actually creating an ID out of it.
    fn is_id(&self) -> Result<(), IdError>;
}

impl UuidValidator for str {
    #[inline] fn as_id(&self) -> Result<String, IdError> { as_id(self) }
    #[inline] fn is_id(&self) -> Result<(), IdError> { is_id(self) }
}

pub trait UuidCore {
    fn with_uuid(&self) -> String;
}

pub trait TryAttachUuid<T> {
    fn maybe_with_uuid(&self) -> Option<T>;
}

pub trait Uuid<'a> : UuidCore {
    fn no_uuid(&'a self) -> &'a str;
    fn re_uuid(&self) -> String;
}

/// Append UUID if no such yet.
fn append_uuid(value: &str) -> String {
    if UUID_RE.is_match(value) {
        return value.into()
    }
    append_uuid_bypass(value)
}

fn append_uuid_bypass(value: &str) -> String {
    format!("{value}-{}", uuid::Uuid::new_v4())
}

pub trait StrUuid {
    /// Show UUID of `self`? — yes/no.
    fn show_uuid(&self, yn: bool) -> &str;
}
impl StrUuid for str {
    /// "Hide" UUID part if present when `show` is `false`.
    fn show_uuid(&self, show: bool) -> &str {
        if show { self } else {
            if UUID_RE_DELIM.is_match(self) {
                &self[..self.len() - 37]// chop '-' delimiter and the UUID which follows
            } else {self}
        }
    }
}

impl UuidCore for str {
    #[inline] fn with_uuid(&self) -> String { append_uuid(self) }
}

impl<'a> Uuid<'a> for str {
    #[inline] fn no_uuid(&'a self) -> &'a str { self.show_uuid(false) }
    #[inline] fn re_uuid(&self) -> String { append_uuid_bypass(self.show_uuid(false)) }
}

impl TryAttachUuid<Item> for Option<Item> {
    fn maybe_with_uuid(&self) -> Option<Item> {
        match self {
            Some(item) => {
                let mut new = item.clone();
                new.set_id(&item.id().re_uuid(), true).ok();
                Some(new)
            }
            None => None
        }
    }
}

/// Get representation as file/hash ready `id`, if possible…
/// 
/// # Args
/// - `input` to be sanitized.
pub fn as_id(input: &str) -> Result<String, IdError> {
    let mut out = String::new();
    let mut last_was_junk = false;
    let mut has_alnum = false;

    for ch in input.trim().to_lowercase().nfd() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_junk = false;
            has_alnum = true;
            continue;
        }

        if !last_was_junk && !out.is_empty() {
            out.push(match ch {
                '-' => '-',
                _ if ch.is_whitespace() => '-',
                _ => '_'
            });
            last_was_junk = true;
        }
    }

    let out = out.trim_end_matches(|c| c == '-' || c == '_');

    if !has_alnum || out.is_empty() {
        return Err(IdError::EmptyOrGarbage);
    }

    if out.len() > super::MAX_ID_LEN {
        return Err(IdError::TooLong);
    }

    for token in out.split(|c| c == '-'||c == '_') {
        if token.is_empty() { continue; }// redundant likely, but …
        if crate::r#const::HARDCODED_RESERVED.contains(token) ||
            VILLAIN_ID.contains(&token)
        {
            log::warn!("ID '{out}' blocked: contains reserved token '{token}'");
            return Err(IdError::ReservedName(out.into()));
        }
    }

    Ok(out.into())
}

/// Check if string is valid as ID without actually creating an ID out of it.
/// 
/// Note#1: unlike [as_id], [is_id] doesn't check against hardcoded reserved words.
/// 
/// Note#2: used mainly by the companion proc-macros.
pub fn is_id(input: &str) -> Result<(), IdError> {
    let mut out = 0;
    let mut last_was_junk = false;
    let mut has_alnum = false;

    for ch in input.trim().to_lowercase().nfd() {
        if ch.is_ascii_alphanumeric() {
            out += 1;
            last_was_junk = false;
            has_alnum = true;
            continue;
        }

        if !last_was_junk && out != 0 {
            out += 1;
            last_was_junk = true;
        }
    }

    if !has_alnum || out == 0 {
        return Err(IdError::EmptyOrGarbage);
    }

    if out > super::MAX_ID_LEN {
        return Err(IdError::TooLong);
    }

    Ok(())
}
