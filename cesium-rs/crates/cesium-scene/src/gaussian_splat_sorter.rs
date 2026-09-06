//! Ported from `packages/engine/Source/Scene/GaussianSplatSorter.js`.

/// Sorts Gaussian splats.
///
/// Sorts splats by depth for correct alpha blending.
pub struct GaussianSplatSorter {
    /// Whether sorting is in progress.
    pub sorting: bool,
}

impl GaussianSplatSorter {
    /// Creates a new GaussianSplatSorter.
    pub fn new() -> Self { Self { sorting: false } }
}

impl Default for GaussianSplatSorter {
    fn default() -> Self { Self::new() }
}
