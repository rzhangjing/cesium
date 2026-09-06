//! Ported from `packages/engine/Source/Scene/EquirectangularPanorama.js`.

/// Equirectangular panorama.
///
/// Represents a panorama as a single equirectangular image.
pub struct EquirectangularPanorama {
    /// The image URL.
    pub url: String,
    /// Whether the panorama is loaded.
    pub loaded: bool,
}

impl EquirectangularPanorama {
    /// Creates a new EquirectangularPanorama.
    pub fn new() -> Self { Self { url: String::new(), loaded: false } }
}

impl Default for EquirectangularPanorama {
    fn default() -> Self { Self::new() }
}
