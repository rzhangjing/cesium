//! Ported from `packages/engine/Source/Scene/GaussianSplatPrimitive.js`.

/// Gaussian splat primitive.
///
/// Renders 3D Gaussian splats for neural radiance field visualization.
pub struct GaussianSplatPrimitive {
    /// Whether the primitive is visible.
    pub show: bool,
    /// The number of splats.
    pub splat_count: u32,
    /// Whether the primitive is ready.
    pub ready: bool,
}

impl GaussianSplatPrimitive {
    /// Creates a new GaussianSplatPrimitive.
    pub fn new() -> Self { Self { show: true, splat_count: 0, ready: false } }
}

impl Default for GaussianSplatPrimitive {
    fn default() -> Self { Self::new() }
}
