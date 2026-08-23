//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetBaseTraversal.js`.

/// Base traversal for 3D Tiles.
pub struct Cesium3DTilesetBaseTraversal {
    _private: (),
}

impl Cesium3DTilesetBaseTraversal {
    /// Creates a new Cesium3DTilesetBaseTraversal.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesetBaseTraversal {
    fn default() -> Self { Self::new() }
}
