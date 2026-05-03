//! Gender of things.

pub enum GenderType {
    NotApplicable,
    Female,
    Male,
}

pub trait Gender {
    fn gender(&self) -> GenderType;
}