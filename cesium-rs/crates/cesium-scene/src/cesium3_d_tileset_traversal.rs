//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetTraversal.js`.

/// Traversal strategy for 3D Tiles tilesets.
pub struct Cesium3DTilesetTraversal {
    _private: (),
}

impl Cesium3DTilesetTraversal {
    /// Creates a new Cesium3DTilesetTraversal.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesetTraversal {
    fn default() -> Self { Self::new() }
}
