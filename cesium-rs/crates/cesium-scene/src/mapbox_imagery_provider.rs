//! Ported from `packages/engine/Source/Scene/MapboxImageryProvider.js`.

/// Imagery provider for Mapbox tiles.
///
/// Loads map tiles from the Mapbox API.
pub struct MapboxImageryProvider {
    /// The Mapbox access token.
    pub access_token: Option<String>,
    /// The Mapbox map ID.
    pub map_id: String,
    /// The tile URL template.
    pub url: String,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl MapboxImageryProvider {
    /// Creates a new MapboxImageryProvider.
    pub fn new() -> Self {
        Self {
            access_token: None,
            map_id: "mapbox.streets".to_string(),
            url: "https://api.mapbox.com/v4/".to_string(),
            ready: false,
        }
    }
}

impl Default for MapboxImageryProvider {
    fn default() -> Self { Self::new() }
}
