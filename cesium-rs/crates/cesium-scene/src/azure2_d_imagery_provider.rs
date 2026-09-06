//! Ported from `packages/engine/Source/Scene/AzureImageryProvider.js`.

/// Imagery provider for Azure Maps.
///
/// Loads map tiles from Azure Maps REST API.
pub struct Azure2DImageryProvider {
    /// The Azure Maps subscription key.
    pub subscription_key: Option<String>,
    /// The tile URL template.
    pub url: String,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl Azure2DImageryProvider {
    /// Creates a new Azure2DImageryProvider.
    pub fn new() -> Self {
        Self {
            subscription_key: None,
            url: "https://atlas.microsoft.com/map/tile".to_string(),
            ready: false,
        }
    }
}

impl Default for Azure2DImageryProvider {
    fn default() -> Self { Self::new() }
}
