//! Ported from `packages/engine/Source/Core/CesiumTerrainProvider.js`.

/// A terrain provider using Cesium terrain tiles.
pub struct CesiumTerrainProvider {
    _private: (),
}

impl CesiumTerrainProvider {
    /// Creates a new CesiumTerrainProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CesiumTerrainProvider {
    fn default() -> Self { Self::new() }
}
