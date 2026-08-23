//! Ported from `packages/engine/Source/Scene/CubeMapPanorama.js`.

/// A cube map panorama.
pub struct CubeMapPanorama {
    _private: (),
}

impl CubeMapPanorama {
    /// Creates a new CubeMapPanorama.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CubeMapPanorama {
    fn default() -> Self { Self::new() }
}
