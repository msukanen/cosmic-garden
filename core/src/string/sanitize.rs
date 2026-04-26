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
