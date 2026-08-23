//! Ported from `packages/engine/Source/Scene/GaussianSplatRenderResources.js`.

/// Resources for Gaussian splat rendering.
pub struct GaussianSplatRenderResources {
    _private: (),
}

impl GaussianSplatRenderResources {
    /// Creates a new GaussianSplatRenderResources.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GaussianSplatRenderResources {
    fn default() -> Self { Self::new() }
}
