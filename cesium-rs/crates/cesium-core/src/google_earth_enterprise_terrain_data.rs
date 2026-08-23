//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTerrainData.js`.

/// Terrain data from Google Earth Enterprise.
pub struct GoogleEarthEnterpriseTerrainData {
    _private: (),
}

impl GoogleEarthEnterpriseTerrainData {
    /// Creates a new GoogleEarthEnterpriseTerrainData.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleEarthEnterpriseTerrainData {
    fn default() -> Self { Self::new() }
}
