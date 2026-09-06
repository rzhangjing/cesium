//! Ported from `packages/engine/Source/Scene/GaussianSplat3DTileContent.js`.

/// Gaussian splat 3D tile content.
///
/// Contains Gaussian splat data within a 3D tile.
pub struct GaussianSplat3DTileContent {
    /// The number of splats.
    pub splat_count: u32,
    /// Whether the content is ready.
    pub ready: bool,
}

impl GaussianSplat3DTileContent {
    /// Creates a new GaussianSplat3DTileContent.
    pub fn new() -> Self { Self { splat_count: 0, ready: false } }
}

impl Default for GaussianSplat3DTileContent {
    fn default() -> Self { Self::new() }
}
