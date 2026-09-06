//! Ported from `packages/engine/Source/Scene/Geometry3DTileContent.js`.

/// Geometry 3D tile content.
///
/// Contains geometry data within a 3D tile.
pub struct Geometry3DTileContent {
    /// The number of features.
    pub features_length: u32,
    /// Whether the content is ready.
    pub ready: bool,
}

impl Geometry3DTileContent {
    /// Creates a new Geometry3DTileContent.
    pub fn new() -> Self { Self { features_length: 0, ready: false } }
}

impl Default for Geometry3DTileContent {
    fn default() -> Self { Self::new() }
}
