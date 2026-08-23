//! Ported from `packages/engine/Source/Scene/Model/TextureUniform.js`.

/// A texture uniform for shaders.
pub struct TextureUniform {
    _private: (),
}

impl TextureUniform {
    /// Creates a new TextureUniform.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TextureUniform {
    fn default() -> Self { Self::new() }
}
