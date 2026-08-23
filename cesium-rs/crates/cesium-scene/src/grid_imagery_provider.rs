//! Ported from `packages/engine/Source/Scene/GridImageryProvider.js`.

/// Grid imagery provider.
pub struct GridImageryProvider {
    _private: (),
}

impl GridImageryProvider {
    /// Creates a new GridImageryProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GridImageryProvider {
    fn default() -> Self { Self::new() }
}
