//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetMostDetailedTraversal.js`.

/// Most-detailed traversal strategy for 3D tilesets.
///
/// Traverses to the most detailed available tiles for maximum quality.
pub struct Cesium3DTilesetMostDetailedTraversal {
    /// The maximum number of tiles visited per frame.
    pub max_visits_per_frame: u32,
    /// Number of tiles visited in the last traversal.
    pub tiles_visited: u32,
}

impl Cesium3DTilesetMostDetailedTraversal {
    /// Creates a new Cesium3DTilesetMostDetailedTraversal.
    pub fn new() -> Self { Self { max_visits_per_frame: 5000, tiles_visited: 0 } }
}

impl Default for Cesium3DTilesetMostDetailedTraversal {
    fn default() -> Self { Self::new() }
}
