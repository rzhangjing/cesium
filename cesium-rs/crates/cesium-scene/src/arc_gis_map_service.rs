//! Ported from `packages/engine/Source/Scene/ArcGisMapServerImageryProvider.js`.

/// ArcGIS map service imagery provider.
///
/// Loads map tiles from an ArcGIS MapServer REST endpoint.
pub struct ArcGisMapService {
    /// The ArcGIS server URL.
    pub url: String,
    /// The map service identifier.
    pub service_id: Option<String>,
    /// Whether the provider is ready.
    pub ready: bool,
    /// The maximum zoom level.
    pub maximum_level: Option<u32>,
}

impl ArcGisMapService {
    /// Creates a new ArcGisMapService.
    pub fn new() -> Self {
        Self {
            url: "https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer".to_string(),
            service_id: None, ready: false, maximum_level: None,
        }
    }
}

impl Default for ArcGisMapService {
    fn default() -> Self { Self::new() }
}
