//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetMostDetailedTraversal.js`.

/// Most detailed traversal for 3D Tiles.
pub struct Cesium3DTilesetMostDetailedTraversal {
    _private: (),
}

impl Cesium3DTilesetMostDetailedTraversal {
    /// Creates a new Cesium3DTilesetMostDetailedTraversal.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesetMostDetailedTraversal {
    fn default() -> Self { Self::new() }
}
