//! Ported from `packages/engine/Source/Scene/GaussianSplatPrimitive.js`.

/// A Gaussian splat primitive.
pub struct GaussianSplatPrimitive {
    _private: (),
}

impl GaussianSplatPrimitive {
    /// Creates a new GaussianSplatPrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GaussianSplatPrimitive {
    fn default() -> Self { Self::new() }
}
