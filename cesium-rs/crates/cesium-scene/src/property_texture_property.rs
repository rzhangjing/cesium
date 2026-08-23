//! Ported from `packages/engine/Source/Scene/PropertyTextureProperty.js`.

/// A property within a property texture.
pub struct PropertyTextureProperty {
    _private: (),
}

impl PropertyTextureProperty {
    /// Creates a new PropertyTextureProperty.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PropertyTextureProperty {
    fn default() -> Self { Self::new() }
}
