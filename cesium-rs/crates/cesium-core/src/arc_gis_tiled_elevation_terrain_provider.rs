//! Ported from `packages/engine/Source/Core/ArcGISTiledElevationTerrainProvider.js`.
//!
//! Terrain provider using ArcGIS tiled elevation service.

use crate::rectangle::Rectangle;

/// A terrain provider using ArcGIS tiled elevation service.
/// Skeleton: requires network I/O.
pub struct ArcGISTiledElevationTerrainProvider {
    _url: String,
}

impl ArcGISTiledElevationTerrainProvider {
    /// Creates a new provider from options.
    pub fn new(url: String) -> Self {
        Self { _url: url }
    }

    /// Requests terrain data for a given tile.
    pub fn request_tile_data(
        &self,
        _x: i32,
        _y: i32,
        _level: i32,
    ) -> Option<()> {
        None // Skeleton
    }

    /// Returns the tiling scheme.
    pub fn tiling_scheme(&self) -> Option<String> {
        None // Skeleton
    }

    /// Returns the availability of terrain data.
    pub fn availability(&self) -> Option<String> {
        None
    }

    /// Returns the error event.
    pub fn error_event(&self) -> Option<String> {
        None
    }

    /// Whether the provider is ready.
    pub fn ready(&self) -> bool {
        false
    }

    /// Whether the provider has vertex normals.
    pub fn has_vertex_normals(&self) -> bool {
        false
    }

    /// Whether the provider allows sampling.
    pub fn has_water_mask(&self) -> bool {
        false
    }

    /// Returns the rectangle of the terrain provider.
    pub fn rectangle(&self) -> Rectangle {
        Rectangle::default()
    }
}
