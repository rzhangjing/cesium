//! Ported from `packages/engine/Source/Scene/GaussianSplatSorter.js`.

/// Sorts Gaussian splats.
pub struct GaussianSplatSorter {
    _private: (),
}

impl GaussianSplatSorter {
    /// Creates a new GaussianSplatSorter.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GaussianSplatSorter {
    fn default() -> Self { Self::new() }
}
