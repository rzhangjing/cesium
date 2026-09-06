//! Ported from `packages/engine/Source/Scene/GoogleEarthEnterpriseImageryProvider.js`.

/// Google Earth Enterprise imagery provider.
///
/// Loads imagery from a Google Earth Enterprise database.
pub struct GoogleEarthEnterpriseImageryProvider {
    /// The server URL.
    pub url: String,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl GoogleEarthEnterpriseImageryProvider {
    /// Creates a new GoogleEarthEnterpriseImageryProvider.
    pub fn new() -> Self { Self { url: String::new(), ready: false } }
}

impl Default for GoogleEarthEnterpriseImageryProvider {
    fn default() -> Self { Self::new() }
}
