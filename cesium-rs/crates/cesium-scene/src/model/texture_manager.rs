//! Ported from `packages/engine/Source/Scene/TextureManager.js`.

/// Manages textures for a model.
pub struct TextureManager {
    pub texture_count: usize,
}

impl TextureManager {
    pub fn new() -> Self { Self { texture_count: 0 } }
}

impl Default for TextureManager {
    fn default() -> Self { Self::new() }
}
