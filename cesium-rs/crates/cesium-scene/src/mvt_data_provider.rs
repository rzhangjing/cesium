//! Ported from `packages/engine/Source/Scene/MVTDataProvider.js`.

/// MVT (Mapbox Vector Tile) data provider.
///
/// Loads and parses Mapbox Vector Tile format data.
pub struct MvtDataProvider {
    /// The data URL.
    pub url: String,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl MvtDataProvider {
    /// Creates a new MvtDataProvider.
    pub fn new() -> Self { Self { url: String::new(), ready: false } }
}

impl Default for MvtDataProvider {
    fn default() -> Self { Self::new() }
}
