//! Ported from `packages/engine/Source/Scene/GaussianSplatTextureGenerator.js`.

/// Generates Gaussian splat textures.
pub struct GaussianSplatTextureGenerator {
    _private: (),
}

impl GaussianSplatTextureGenerator {
    /// Creates a new GaussianSplatTextureGenerator.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GaussianSplatTextureGenerator {
    fn default() -> Self { Self::new() }
}
