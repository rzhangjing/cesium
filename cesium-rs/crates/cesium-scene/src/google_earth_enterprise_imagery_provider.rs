//! Ported from `packages/engine/Source/Scene/GoogleEarthEnterpriseImageryProvider.js`.

/// Google Earth Enterprise imagery provider.
pub struct GoogleEarthEnterpriseImageryProvider {
    _private: (),
}

impl GoogleEarthEnterpriseImageryProvider {
    /// Creates a new GoogleEarthEnterpriseImageryProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleEarthEnterpriseImageryProvider {
    fn default() -> Self { Self::new() }
}
