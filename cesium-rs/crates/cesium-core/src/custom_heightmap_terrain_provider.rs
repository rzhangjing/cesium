//! Ported from `packages/engine/Source/Core/CustomHeightmapTerrainProvider.js`.

/// A terrain provider using custom heightmap data.
pub struct CustomHeightmapTerrainProvider {
    _private: (),
}

impl CustomHeightmapTerrainProvider {
    /// Creates a new CustomHeightmapTerrainProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CustomHeightmapTerrainProvider {
    fn default() -> Self { Self::new() }
}
