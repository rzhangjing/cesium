//! Ported from `packages/engine/Source/Renderer/TextureCache.js`.

/// Caches textures for reuse across frames.
pub struct TextureCache {
    _private: (),
}

impl TextureCache {
    /// Creates a new TextureCache.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TextureCache {
    fn default() -> Self { Self::new() }
}
