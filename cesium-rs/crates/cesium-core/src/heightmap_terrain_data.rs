//! Ported from `packages/engine/Source/Core/HeightmapTerrainData.js`.

/// Terrain data from a heightmap.
pub struct HeightmapTerrainData {
    _private: (),
}

impl HeightmapTerrainData {
    /// Creates a new HeightmapTerrainData.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for HeightmapTerrainData {
    fn default() -> Self { Self::new() }
}
