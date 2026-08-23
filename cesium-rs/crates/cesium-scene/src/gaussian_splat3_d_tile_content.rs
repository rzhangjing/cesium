//! Ported from `packages/engine/Source/Scene/GaussianSplat3DTileContent.js`.

/// Gaussian splat 3D tile content.
pub struct GaussianSplat3DTileContent {
    _private: (),
}

impl GaussianSplat3DTileContent {
    /// Creates a new GaussianSplat3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GaussianSplat3DTileContent {
    fn default() -> Self { Self::new() }
}
