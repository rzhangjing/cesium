//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetHeatmap.js`.

/// Heatmap visualization for 3D tilesets.
///
/// Generates color-coded visualization of tile properties (SSE, depth, etc.).
pub struct Cesium3DTilesetHeatmap {
    /// The heatmap tile property name being visualized.
    pub heatmap_tile_property_name: Option<String>,
    /// Whether the heatmap is dirty and needs recomputation.
    pub dirty: bool,
}

impl Cesium3DTilesetHeatmap {
    /// Creates a new Cesium3DTilesetHeatmap.
    pub fn new() -> Self { Self { heatmap_tile_property_name: None, dirty: true } }
}

impl Default for Cesium3DTilesetHeatmap {
    fn default() -> Self { Self::new() }
}
