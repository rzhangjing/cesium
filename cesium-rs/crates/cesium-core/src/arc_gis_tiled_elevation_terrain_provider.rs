//! Ported from `packages/engine/Source/Core/ArcGISTiledElevationTerrainProvider.js`.

/// A terrain provider using ArcGIS tiled elevation data.
pub struct ArcGISTiledElevationTerrainProvider {
    _private: (),
}

impl ArcGISTiledElevationTerrainProvider {
    /// Creates a new ArcGISTiledElevationTerrainProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ArcGISTiledElevationTerrainProvider {
    fn default() -> Self { Self::new() }
}
