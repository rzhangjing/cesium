//! Ported from `packages/engine/Source/Core/GoogleMaps.js`.
//!
//! Default settings for accessing the Google Maps API.

/// Default Google Maps API endpoint.
pub const MAP_TILES_API_ENDPOINT: &str = "https://tile.googleapis.com/";

/// Default Google Street View Static API endpoint.
pub const STREET_VIEW_STATIC_API_ENDPOINT: &str =
    "https://maps.googleapis.com/maps/api/streetview";

/// Default settings for Google Maps API access.
#[derive(Debug, Clone)]
pub struct GoogleMaps {
    /// The API key (None if not set).
    pub default_api_key: Option<String>,
    /// The map tiles API endpoint.
    pub map_tiles_api_endpoint: String,
    /// The Street View Static API key (None if not set).
    pub default_street_view_static_api_key: Option<String>,
    /// The Street View Static API endpoint.
    pub street_view_static_api_endpoint: String,
}

impl Default for GoogleMaps {
    fn default() -> Self {
        Self {
            default_api_key: None,
            map_tiles_api_endpoint: MAP_TILES_API_ENDPOINT.to_string(),
            default_street_view_static_api_key: None,
            street_view_static_api_endpoint: STREET_VIEW_STATIC_API_ENDPOINT.to_string(),
        }
    }
}
