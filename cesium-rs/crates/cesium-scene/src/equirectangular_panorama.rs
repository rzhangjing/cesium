//! Ported from `packages/engine/Source/Scene/EquirectangularPanorama.js`.

/// An equirectangular panorama.
pub struct EquirectangularPanorama {
    _private: (),
}

impl EquirectangularPanorama {
    /// Creates a new EquirectangularPanorama.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for EquirectangularPanorama {
    fn default() -> Self { Self::new() }
}
