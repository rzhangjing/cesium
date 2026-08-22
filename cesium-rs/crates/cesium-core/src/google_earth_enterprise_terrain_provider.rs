//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTerrainProvider.js`.
//!
//! Terrain provider using Google Earth Enterprise.

use crate::rectangle::Rectangle;

/// A terrain provider using Google Earth Enterprise.
/// Skeleton: requires network I/O.
pub struct GoogleEarthEnterpriseTerrainProvider {
    _url: String,
}

impl GoogleEarthEnterpriseTerrainProvider {
    /// Creates a new provider from options.
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
}
