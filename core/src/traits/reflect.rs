//! Reflect, mirror, almost-clone-but-not-quite.
pub trait Reflector {
    fn reflect(&self) -> Self;
    fn deep_reflect(&self) -> Self;
}
