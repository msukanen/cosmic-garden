//! Sanitize strings…
/// A trait for anything that wants sanitize…
pub trait Sanitizer {
    fn sanitize(&self) -> String;
}

impl Sanitizer for &str {
    /// Sanitize away all "control" characters.
    fn sanitize(&self) -> String {
        self.trim().chars()
            .filter(|c| !c.is_control())
            .collect::<String>()
    }
}

impl Sanitizer for String {
    fn sanitize(&self) -> String { self.as_str().sanitize()}
}

impl Sanitizer for &String {
    fn sanitize(&self) -> String { self.as_str().sanitize()}
}

/// Get a slice of `s` w/o its last letter.
pub(crate) fn clip_last_char<'a>(s: &'a str) -> &'a str {
    s.char_indices()
        .rev()
        .nth(0)
        .map(|(idx, _)| &s[..idx])
        .unwrap_or("")
}

#[cfg(test)]
mod sanitize_tests {
    #[test]
    fn clip_last_char() {
        let abc = "đa¡bč";
        let ab = super::clip_last_char(abc);
        assert_eq!("đa¡b", ab);
    }
}
