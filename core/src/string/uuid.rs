//! UUID related things…

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub(crate) static ref UUID_RE: Regex = Regex::new(
        r"(?P<uuid>[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$"
    ).unwrap();
}

pub trait Uuid {
    fn with_uuid(&self) -> String;
    fn no_uuid(&self) -> String;
    fn re_uuid(&self) -> String { self.no_uuid().with_uuid() }
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
    if let Some(caps) = UUID_RE.captures(value) {
        let uuid_str = caps.name("uuid").unwrap().as_str();
        return value
            .replace(uuid_str, "")
            .trim_end_matches('-')
            .into();
    }
    value.into()
}

impl Uuid for &str {
    #[inline]
    fn with_uuid(&self) -> String { append_uuid(self) }
    #[inline]
    fn no_uuid(&self) -> String { remove_uuid(self) }
}

impl Uuid for String {
    #[inline]
    fn with_uuid(&self) -> String { append_uuid(self) }
    #[inline]
    fn no_uuid(&self) -> String { remove_uuid(self) }
}

impl Uuid for &String {
    #[inline]
    fn with_uuid(&self) -> String { append_uuid(self) }
    #[inline]
    fn no_uuid(&self) -> String { remove_uuid(self) }
}
