//! Ported from `packages/engine/Source/Scene/Cesium3DTileOptimizations.js`.

/// Tracks optimization state for a 3D tile.
///
/// Stores flags used during traversal to skip unnecessary processing.
pub struct Cesium3DTileOptimizations {
    /// Whether the tile uses content union optimization.
    pub uses_content_union: bool,
}

impl Cesium3DTileOptimizations {
    /// Creates a new Cesium3DTileOptimizations.
    pub fn new() -> Self { Self { uses_content_union: false } }
}

impl Default for Cesium3DTileOptimizations {
    fn default() -> Self { Self::new() }
}
