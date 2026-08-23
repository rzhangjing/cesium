//! Ported from `packages/engine/Source/Scene/ImageBasedLighting.js`.

/// Image-based lighting.
pub struct ImageBasedLighting {
    _private: (),
}

impl ImageBasedLighting {
    /// Creates a new ImageBasedLighting.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageBasedLighting {
    fn default() -> Self { Self::new() }
}
