//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetHeatmap.js`.

/// Heatmap visualization for 3D Tiles.
pub struct Cesium3DTilesetHeatmap {
    _private: (),
}

impl Cesium3DTilesetHeatmap {
    /// Creates a new Cesium3DTilesetHeatmap.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesetHeatmap {
    fn default() -> Self { Self::new() }
}
