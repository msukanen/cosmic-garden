//! UUID related things…

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub(crate) static ref UUID_RE: Regex = Regex::new(
        r"(?P<uuid>[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$"
    ).unwrap();
    pub(crate) static ref UUID_RE_DELIM: Regex = Regex::new(
        r"[-][0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
    ).unwrap();
}

pub trait Uuid {
    fn with_uuid(&self) -> String;
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

impl Uuid for &str {
    #[inline] fn with_uuid(&self) -> String { append_uuid(self) }
    #[inline] fn no_uuid(&self) -> String { remove_uuid(self) }
    #[inline] fn re_uuid(&self) -> String { append_uuid(self.show_uuid(false)) }
}

impl Uuid for String {
    #[inline] fn with_uuid(&self) -> String { append_uuid(self) }
    #[inline] fn no_uuid(&self) -> String { remove_uuid(self) }
    #[inline] fn re_uuid(&self) -> String { append_uuid(self.show_uuid(false)) }
}

impl Uuid for &String {
    #[inline] fn with_uuid(&self) -> String { append_uuid(self) }
    #[inline] fn no_uuid(&self) -> String { remove_uuid(self) }
    #[inline] fn re_uuid(&self) -> String { append_uuid(self.show_uuid(false)) }
}
