//! Ported from `packages/engine/Source/Scene/Geometry3DTileContent.js`.

/// Geometry 3D tile content.
pub struct Geometry3DTileContent {
    _private: (),
}

impl Geometry3DTileContent {
    /// Creates a new Geometry3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Geometry3DTileContent {
    fn default() -> Self { Self::new() }
}
