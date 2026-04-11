//! Descriptionable things…

pub trait Describable {
    /// Get current description.
    fn desc<'a>(&'a self) -> &'a str;
    /// Set description, if possible.
    /// 
    /// # Args
    /// - `text`— new description.
    /// 
    /// # Returns
    /// `true` if actually set.
    fn set_desc(&mut self, text: &str) -> bool;
}
