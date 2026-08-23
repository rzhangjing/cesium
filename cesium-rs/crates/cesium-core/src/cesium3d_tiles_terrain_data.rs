//! Ported from `packages/engine/Source/Core/Cesium3DTilesTerrainData.js`.

/// Terrain data from 3D Tiles.
pub struct Cesium3DTilesTerrainData {
    _private: (),
}

impl Cesium3DTilesTerrainData {
    /// Creates a new Cesium3DTilesTerrainData.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesTerrainData {
    fn default() -> Self { Self::new() }
}
