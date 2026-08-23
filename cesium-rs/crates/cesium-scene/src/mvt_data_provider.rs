//! Ported from `packages/engine/Source/Scene/MvtDataProvider.js`.

/// A data provider for MVT (Mapbox Vector Tiles) data.
pub struct MvtDataProvider {
    _private: (),
}

impl MvtDataProvider {
    /// Creates a new MvtDataProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MvtDataProvider {
    fn default() -> Self { Self::new() }
}
