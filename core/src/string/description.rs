//! Descriptionable things…

pub trait Describable {
    fn desc<'a>(&'a self) -> &'a str;
}
