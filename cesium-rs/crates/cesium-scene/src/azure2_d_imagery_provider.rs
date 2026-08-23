//! Ported from `packages/engine/Source/Scene/Azure2DImageryProvider.js`.

/// An imagery provider for Azure Maps 2D tiles.
pub struct Azure2DImageryProvider {
    _private: (),
}

impl Azure2DImageryProvider {
    /// Creates a new Azure2DImageryProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Azure2DImageryProvider {
    fn default() -> Self { Self::new() }
}
