//! Ported from `packages/engine/Source/Scene/Multiple3DTileContent.js`.

/// Multiple 3D Tiles content in a single tile.
pub struct Multiple3DTileContent {
    _private: (),
}

impl Multiple3DTileContent {
    /// Creates a new Multiple3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Multiple3DTileContent {
    fn default() -> Self { Self::new() }
}
