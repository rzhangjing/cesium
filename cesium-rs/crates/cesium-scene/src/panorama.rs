//! Ported from `packages/engine/Source/Scene/Panorama.js`.

/// A panoramic image.
pub struct Panorama {
    _private: (),
}

impl Panorama {
    /// Creates a new Panorama.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Panorama {
    fn default() -> Self { Self::new() }
}
