//! UUID related things…

use lazy_static::lazy_static;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::item::Item;
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

impl UuidValidator for String {
    #[inline] fn as_id(&self) -> Result<String, IdError> { as_id(self) }
    #[inline] fn is_id(&self) -> Result<(), IdError> { is_id(self) }
}

impl UuidValidator for &String {
    #[inline] fn as_id(&self) -> Result<String, IdError> { as_id(self) }
    #[inline] fn is_id(&self) -> Result<(), IdError> { is_id(self) }
}

impl UuidValidator for &str {
    #[inline] fn as_id(&self) -> Result<String, IdError> { as_id(self) }
    #[inline] fn is_id(&self) -> Result<(), IdError> { is_id(self) }
}

pub trait UuidCore {
    fn with_uuid(&self) -> String;
}

pub trait TryAttachUuid<T> {
    fn maybe_with_uuid(&self) -> Option<T>;
}

pub trait Uuid : UuidCore {
    fn no_uuid(&self) -> String;
    fn re_uuid(&self) -> String;
}

/// Append UUID if no such yet.
fn append_uuid(value: &str) -> String {
    if UUID_RE.is_match(value) {
        return value.into()
    }
    format!("{value}-{}", uuid::Uuid::new_v4())
}

/// Remove UUID if present.
pub fn remove_uuid(value: &str) -> String {
    value.show_uuid(false).into()
}

pub trait StrUuid {
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

impl UuidCore for &str {
    #[inline] fn with_uuid(&self) -> String { append_uuid(self) }
}
impl Uuid for &str {
    #[inline] fn no_uuid(&self) -> String { remove_uuid(self) }
    #[inline] fn re_uuid(&self) -> String { append_uuid(self.show_uuid(false)) }
}

impl UuidCore for String {
    #[inline] fn with_uuid(&self) -> String { append_uuid(self) }
}
impl Uuid for String {
    #[inline] fn no_uuid(&self) -> String { remove_uuid(self) }
    #[inline] fn re_uuid(&self) -> String { append_uuid(self.show_uuid(false)) }
}

impl UuidCore for &String {
    #[inline] fn with_uuid(&self) -> String { append_uuid(self) }
}
impl Uuid for &String {
    #[inline] fn no_uuid(&self) -> String { remove_uuid(self) }
    #[inline] fn re_uuid(&self) -> String { append_uuid(self.show_uuid(false)) }
}

impl TryAttachUuid<Item> for Option<Item> {
    fn maybe_with_uuid(&self) -> Option<Item> {
        match self {
            Some(item) => {
                let mut new = item.clone();
                *(new.id_mut()) = item.id().re_uuid();
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

    if !has_alnum || out.is_empty() {
        return Err(IdError::EmptyOrGarbage);
    }

    if out.len() > super::MAX_ID_LEN {
        return Err(IdError::TooLong);
    }

    if crate::r#const::HARDCODED_RESERVED.contains(out.as_str()) {
        return Err(IdError::ReservedName(out.clone()));
    }

    Ok(out)
}

/// Check if string is valid as ID without actually creating an ID out of it.
/// 
/// Note: unlike [as_id], [is_id] doesn't check against hardcoded reserved words.
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

/// `as_id`, but with UUID attached (if not present already).
/// 
/// # Args
/// - `value` with or without UUID.
/// 
/// # Returns
/// Either UUID'ed `value` (or near so) or [`IdError`][IdError].
pub fn as_id_with_uuid(value: &str) -> Result<String, IdError> {
    let base_id = value.as_id()?;
    Ok(base_id.with_uuid())
}
