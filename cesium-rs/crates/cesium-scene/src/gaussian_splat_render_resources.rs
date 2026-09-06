//! Ported from `packages/engine/Source/Scene/GaussianSplatRenderResources.js`.

/// Gaussian splat render resources.
///
/// Manages GPU resources for Gaussian splat rendering.
pub struct GaussianSplatRenderResources {
    /// Whether resources are allocated.
    pub allocated: bool,
}

impl GaussianSplatRenderResources {
    /// Creates a new GaussianSplatRenderResources.
    pub fn new() -> Self { Self { allocated: false } }
}

impl Default for GaussianSplatRenderResources {
    fn default() -> Self { Self::new() }
}
