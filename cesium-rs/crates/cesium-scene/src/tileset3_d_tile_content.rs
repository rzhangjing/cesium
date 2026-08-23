//! Ported from `packages/engine/Source/Scene/Tileset3DTileContent.js`.

/// Tileset 3D Tiles content.
pub struct Tileset3DTileContent {
    _private: (),
}

impl Tileset3DTileContent {
    /// Creates a new Tileset3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Tileset3DTileContent {
    fn default() -> Self { Self::new() }
}
