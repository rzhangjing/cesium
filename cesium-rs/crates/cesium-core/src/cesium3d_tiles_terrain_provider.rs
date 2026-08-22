//! Ported from `packages/engine/Source/Core/Cesium3DTilesTerrainProvider.js`.
//!
//! Terrain provider using 3D Tiles.

use crate::rectangle::Rectangle;

/// A terrain provider using 3D Tiles.
/// Skeleton: requires 3D Tiles infrastructure.
pub struct Cesium3DTilesTerrainProvider;

impl Cesium3DTilesTerrainProvider {
    /// Whether the provider is ready.
    pub fn ready(&self) -> bool { false }
    /// Returns the rectangle.
    pub fn rectangle(&self) -> Rectangle { Rectangle::default() }
    /// Whether the provider has vertex normals.
    pub fn has_vertex_normals(&self) -> bool { false }
    /// Whether the provider has water mask.
    pub fn has_water_mask(&self) -> bool { false }
}
