//! Ported from `packages/engine/Source/Scene/GaussianSplatTextureGenerator.js`.

/// Gaussian splat texture generator.
///
/// Generates GPU textures from Gaussian splat data.
pub struct GaussianSplatTextureGenerator {
    /// Whether the generator is ready.
    pub ready: bool,
}

impl GaussianSplatTextureGenerator {
    /// Creates a new GaussianSplatTextureGenerator.
    pub fn new() -> Self { Self { ready: false } }
}

impl Default for GaussianSplatTextureGenerator {
    fn default() -> Self { Self::new() }
}
