//! Ported from `packages/engine/Source/Scene/GoogleStreetViewCubeMapPanoramaProvider.js`.

/// Google Street View cube map panorama provider.
pub struct GoogleStreetViewCubeMapPanoramaProvider {
    _private: (),
}

impl GoogleStreetViewCubeMapPanoramaProvider {
    /// Creates a new GoogleStreetViewCubeMapPanoramaProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleStreetViewCubeMapPanoramaProvider {
    fn default() -> Self { Self::new() }
}
