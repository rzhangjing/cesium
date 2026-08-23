//! Ported from `packages/engine/Source/Scene/Megatexture.js`.

/// A megatexture.
pub struct Megatexture {
    _private: (),
}

impl Megatexture {
    /// Creates a new Megatexture.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Megatexture {
    fn default() -> Self { Self::new() }
}
