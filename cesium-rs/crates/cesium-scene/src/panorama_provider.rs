//! Ported from `packages/engine/Source/Scene/PanoramaProvider.js`.

/// Panorama provider.
///
/// Interface for loading panoramic images.
pub struct PanoramaProvider {
    /// Whether the provider is ready.
    pub ready: bool,
}

impl PanoramaProvider {
    /// Creates a new PanoramaProvider.
    pub fn new() -> Self { Self { ready: false } }
}

impl Default for PanoramaProvider {
    fn default() -> Self { Self::new() }
}
