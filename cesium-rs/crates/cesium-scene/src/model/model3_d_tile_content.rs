//! Ported from `packages/engine/Source/Scene/Model/Model3DTileContent.js`.

/// 3D Tiles content backed by a model.
pub struct Model3DTileContent {
    _private: (),
}

impl Model3DTileContent {
    /// Creates a new Model3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Model3DTileContent {
    fn default() -> Self { Self::new() }
}
