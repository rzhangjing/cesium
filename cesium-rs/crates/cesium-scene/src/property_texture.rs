//! Ported from `packages/engine/Source/Scene/PropertyTexture.js`.

/// A property texture in structured metadata.
pub struct PropertyTexture {
    _private: (),
}

impl PropertyTexture {
    /// Creates a new PropertyTexture.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PropertyTexture {
    fn default() -> Self { Self::new() }
}
