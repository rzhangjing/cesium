//! Ported from `packages/engine/Source/Scene/GoogleStreetViewCubeMapPanoramaProvider.js`.

/// Google Street View cube map panorama provider.
///
/// Loads Street View panoramas as cube maps from Google.
pub struct GoogleStreetViewCubeMapPanoramaProvider {
    /// The API key.
    pub api_key: Option<String>,
    /// Whether the provider is ready.
    pub ready: bool,
}

impl GoogleStreetViewCubeMapPanoramaProvider {
    /// Creates a new GoogleStreetViewCubeMapPanoramaProvider.
    pub fn new() -> Self { Self { api_key: None, ready: false } }
}

impl Default for GoogleStreetViewCubeMapPanoramaProvider {
    fn default() -> Self { Self::new() }
}
