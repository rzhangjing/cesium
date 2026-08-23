//! Ported from `packages/engine/Source/Core/Cesium3DTilesTerrainGeometryProcessor.js`.

/// Processes geometry from 3D Tiles terrain.
pub struct Cesium3DTilesTerrainGeometryProcessor {
    _private: (),
}

impl Cesium3DTilesTerrainGeometryProcessor {
    /// Creates a new Cesium3DTilesTerrainGeometryProcessor.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesTerrainGeometryProcessor {
    fn default() -> Self { Self::new() }
}
