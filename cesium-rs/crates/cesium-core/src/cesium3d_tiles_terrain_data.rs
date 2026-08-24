//! Ported from `packages/engine/Source/Core/Cesium3DTilesTerrainData.js`.
//!
//! # DEVIATION (Scene dependency, deferred)
//!
//! `Cesium3DTilesTerrainData` in JS extends `HeightmapTerrainData` and
//! overrides `createMesh` to call `Cesium3DTileContent` methods that
//! belong to the Scene layer (`Cesium3DTileset`). Since cesium-core must
//! not depend on cesium-scene, this type is kept as a stub. The full
//! port requires the Scene-layer `Cesium3DTileContent` and will be
//! implemented when the Scene crate is available.
//!
//! Registered in `docs/deferred.md`.

use crate::rectangle::Rectangle;
use crate::terrain_data::TerrainData;

/// Terrain data from 3D Tiles content.
///
/// DEVIATION: this is a stub — see module-level documentation.
pub struct Cesium3DTilesTerrainData {
    /// The underlying heightmap-like terrain data (DEVIATION: not yet
    /// connected to a 3D Tiles content source).
    _private: (),
}

impl Cesium3DTilesTerrainData {
    /// Creates a new stub `Cesium3DTilesTerrainData`.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for Cesium3DTilesTerrainData {
    fn default() -> Self {
        Self::new()
    }
}

impl TerrainData for Cesium3DTilesTerrainData {
    fn interpolate_height(&self, _rectangle: &Rectangle, _longitude: f64, _latitude: f64) -> f64 {
        // DEVIATION: stub — returns 0.0 (flat ellipsoid).
        0.0
    }

    fn is_child_available(&self, _this_x: i32, _this_y: i32, _child_x: i32, _child_y: i32) -> bool {
        false
    }

    fn was_created_by_upsampling(&self) -> bool {
        false
    }
}
