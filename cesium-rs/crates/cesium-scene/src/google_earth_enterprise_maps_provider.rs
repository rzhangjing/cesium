//! Ported from `packages/engine/Source/Scene/GoogleEarthEnterpriseMapsProvider.js`.

/// Google Earth Enterprise maps provider.
pub struct GoogleEarthEnterpriseMapsProvider {
    _private: (),
}

impl GoogleEarthEnterpriseMapsProvider {
    /// Creates a new GoogleEarthEnterpriseMapsProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleEarthEnterpriseMapsProvider {
    fn default() -> Self { Self::new() }
}
