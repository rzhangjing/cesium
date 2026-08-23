//! Ported from `packages/engine/Source/Scene/PanoramaProvider.js`.

/// A provider for panoramic images.
pub struct PanoramaProvider {
    _private: (),
}

impl PanoramaProvider {
    /// Creates a new PanoramaProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PanoramaProvider {
    fn default() -> Self { Self::new() }
}
