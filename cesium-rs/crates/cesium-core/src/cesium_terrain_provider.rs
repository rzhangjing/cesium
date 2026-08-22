//! Ported from `packages/engine/Source/Core/CesiumTerrainProvider.js`.
//!
//! Terrain provider using Cesium terrain tiles (quantized-mesh, heightmap, etc).

use crate::rectangle::Rectangle;

/// A terrain provider that uses Cesium terrain server tiles.
/// Skeleton: requires network I/O and terrain tile parsing.
pub struct CesiumTerrainProvider {
    _url: String,
}

impl CesiumTerrainProvider {
    /// Creates a new Cesium terrain provider.
    pub fn new(url: String) -> Self {
        Self { _url: url }
    }

    /// Whether the provider is ready.
    pub fn ready(&self) -> bool { false }
    /// Returns the rectangle.
    pub fn rectangle(&self) -> Rectangle { Rectangle::default() }
    /// Whether the provider has vertex normals.
    pub fn has_vertex_normals(&self) -> bool { false }
    /// Whether the provider has water mask.
    pub fn has_water_mask(&self) -> bool { false }
    /// Whether the provider allows heightmap sampling.
    pub fn has_geodetic_surface_normals(&self) -> bool { false }
}
