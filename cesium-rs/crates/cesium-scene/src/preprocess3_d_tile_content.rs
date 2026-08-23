//! Ported from `packages/engine/Source/Scene/preprocess3DTileContent.js`.

/// Preprocesses 3D Tiles content.
pub struct Preprocess3DTileContent {
    _private: (),
}

impl Preprocess3DTileContent {
    /// Creates a new Preprocess3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Preprocess3DTileContent {
    fn default() -> Self { Self::new() }
}
