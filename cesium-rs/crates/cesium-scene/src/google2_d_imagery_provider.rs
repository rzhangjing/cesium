//! Ported from `packages/engine/Source/Scene/Google2DImageryProvider.js`.

/// Google 2D imagery provider.
pub struct Google2DImageryProvider {
    _private: (),
}

impl Google2DImageryProvider {
    /// Creates a new Google2DImageryProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Google2DImageryProvider {
    fn default() -> Self { Self::new() }
}
