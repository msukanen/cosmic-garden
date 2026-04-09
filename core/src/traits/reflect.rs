//! Reflect, mirror, almost-clone-but-not-quite.
pub trait Reflector {
    fn reflect(&self) -> Self;
}
