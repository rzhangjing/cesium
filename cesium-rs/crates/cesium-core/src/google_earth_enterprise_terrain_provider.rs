//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTerrainProvider.js`.

/// A terrain provider using Google Earth Enterprise.
pub struct GoogleEarthEnterpriseTerrainProvider {
    _private: (),
}

impl GoogleEarthEnterpriseTerrainProvider {
    /// Creates a new GoogleEarthEnterpriseTerrainProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleEarthEnterpriseTerrainProvider {
    fn default() -> Self { Self::new() }
}
