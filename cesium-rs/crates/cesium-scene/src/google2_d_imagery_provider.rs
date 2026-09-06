//! Ported from `packages/engine/Source/Scene/Google2DImageryProvider.js`.

/// Google 2D imagery provider.
///
/// Loads map tiles from Google Maps API.
pub struct Google2DImageryProvider {
    /// The tile URL template.
    pub url: String,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl Google2DImageryProvider {
    /// Creates a new Google2DImageryProvider.
    pub fn new() -> Self {
        Self { url: String::new(), ready: false }
    }
}

impl Default for Google2DImageryProvider {
    fn default() -> Self { Self::new() }
}
