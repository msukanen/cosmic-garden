/// Line-ending related functionality.
pub trait LineEndingExt {
    /// Check if there's newline ending.
    fn ends_with_newline(&self) -> bool;
    /// Ensure that there is a newline ending.
    fn ensure_lf(&self) -> String;
}

impl LineEndingExt for &str {
    /// Check if there's LF or CR ending the text, or whether there's no text at all.
    /// 
    /// # Returns
    /// `false` if there's text but no LF or CR at the end.
    fn ends_with_newline(&self) -> bool {
        self.is_empty() || self.ends_with('\n') || self.ends_with('\r')
    }

    /// Tag LF at end if needed.
    fn ensure_lf(&self) -> String {
        if !self.ends_with_newline() {
            format!("{}\n", self)
        } else {
            self.to_string()
        }
    }
}
