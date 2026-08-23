//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTileInformation.js`.

/// Tile information for Google Earth Enterprise.
pub struct GoogleEarthEnterpriseTileInformation {
    _private: (),
}

impl GoogleEarthEnterpriseTileInformation {
    /// Creates a new GoogleEarthEnterpriseTileInformation.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleEarthEnterpriseTileInformation {
    fn default() -> Self { Self::new() }
}
