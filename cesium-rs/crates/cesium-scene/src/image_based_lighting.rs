//! Ported from `packages/engine/Source/Scene/ImageBasedLighting.js`.

/// Image-based lighting.
///
/// Manages IBL environment maps for PBR rendering.
pub struct ImageBasedLighting {
    /// Whether IBL is enabled.
    pub enabled: bool,
    /// The IBL intensity.
    pub intensity: f32,
}

impl ImageBasedLighting {
    /// Creates a new ImageBasedLighting.
    pub fn new() -> Self { Self { enabled: true, intensity: 1.0 } }
}

impl Default for ImageBasedLighting {
    fn default() -> Self { Self::new() }
}
