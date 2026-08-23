//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetSkipTraversal.js`.

/// Skip traversal for 3D Tiles.
pub struct Cesium3DTilesetSkipTraversal {
    _private: (),
}

impl Cesium3DTilesetSkipTraversal {
    /// Creates a new Cesium3DTilesetSkipTraversal.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesetSkipTraversal {
    fn default() -> Self { Self::new() }
}
