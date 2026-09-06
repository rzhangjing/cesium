//! Ported from `packages/engine/Source/Scene/GoogleEarthEnterpriseMapsProvider.js`.

/// Google Earth Enterprise maps imagery provider.
///
/// Loads map tiles from a Google Earth Enterprise server.
pub struct GoogleEarthEnterpriseMapsProvider {
    /// The server URL.
    pub url: String,
    /// The channel/path for map tiles.
    pub channel: u32,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl GoogleEarthEnterpriseMapsProvider {
    /// Creates a new GoogleEarthEnterpriseMapsProvider.
    pub fn new() -> Self { Self { url: String::new(), channel: 0, ready: false } }
}

impl Default for GoogleEarthEnterpriseMapsProvider {
    fn default() -> Self { Self::new() }
}
