//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetSkipTraversal.js`.

/// Skip traversal strategy for 3D tilesets.
///
/// Optimized traversal that skips tiles below a screen-space error threshold.
pub struct Cesium3DTilesetSkipTraversal {
    /// The maximum number of tiles visited per frame.
    pub max_visits_per_frame: u32,
    /// Number of tiles visited in the last traversal.
    pub tiles_visited: u32,
}

impl Cesium3DTilesetSkipTraversal {
    /// Creates a new Cesium3DTilesetSkipTraversal.
    pub fn new() -> Self { Self { max_visits_per_frame: 5000, tiles_visited: 0 } }
}

impl Default for Cesium3DTilesetSkipTraversal {
    fn default() -> Self { Self::new() }
}
