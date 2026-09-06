//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetBaseTraversal.js`.

/// Base traversal strategy for 3D tilesets.
///
/// Traverses from root to leaves, selecting tiles for rendering and refinement.
pub struct Cesium3DTilesetBaseTraversal {
    /// The maximum number of tiles visited per frame.
    pub max_visits_per_frame: u32,
    /// Number of tiles visited in the last traversal.
    pub tiles_visited: u32,
}

impl Cesium3DTilesetBaseTraversal {
    /// Creates a new Cesium3DTilesetBaseTraversal.
    pub fn new() -> Self { Self { max_visits_per_frame: 5000, tiles_visited: 0 } }
}

impl Default for Cesium3DTilesetBaseTraversal {
    fn default() -> Self { Self::new() }
}
