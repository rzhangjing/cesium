//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseMetadata.js`.

/// Metadata for Google Earth Enterprise terrain.
pub struct GoogleEarthEnterpriseMetadata {
    _private: (),
}

impl GoogleEarthEnterpriseMetadata {
    /// Creates a new GoogleEarthEnterpriseMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleEarthEnterpriseMetadata {
    fn default() -> Self { Self::new() }
}
