//! Ported from `packages/engine/Source/Scene/MapboxStyleImageryProvider.js`.

/// Mapbox Style imagery provider.
///
/// Loads map tiles using Mapbox Style Specification.
pub struct MapboxStyleImageryProvider {
    /// The Mapbox Style ID.
    pub style_id: String,
    /// The Mapbox access token.
    pub access_token: Option<String>,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl MapboxStyleImageryProvider {
    /// Creates a new MapboxStyleImageryProvider.
    pub fn new() -> Self {
        Self { style_id: String::new(), access_token: None, ready: false }
    }
}

impl Default for MapboxStyleImageryProvider {
    fn default() -> Self { Self::new() }
}
