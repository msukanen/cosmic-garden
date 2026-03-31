//! Everything has an identity, let's deal with that…
use cosmic_garden_pm::*;

pub trait IdentityQuery {
    fn id<'a>(&'a self) -> &'a str;
    fn title<'a>(&'a self) -> &'a str;
}

pub trait IdentityMut {
    fn id_mut<'a>(&'a mut self) -> &'a mut String;
    fn title_mut<'a>(&'a mut self) -> &'a mut String;
}

#[cfg(test)]
#[derive(IdentityQuery, IdentityMut)]
struct Identifiable {
    id: String,
    #[identity(title)]
    nomnom: String,
}

#[cfg(test)]
mod identity_tests {
    use super::*;

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
