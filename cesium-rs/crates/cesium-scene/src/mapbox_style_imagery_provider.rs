//! Ported from `packages/engine/Source/Scene/MapboxStyleImageryProvider.js`.

/// Mapbox style imagery provider.
pub struct MapboxStyleImageryProvider {
    _private: (),
}

impl MapboxStyleImageryProvider {
    /// Creates a new MapboxStyleImageryProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MapboxStyleImageryProvider {
    fn default() -> Self { Self::new() }
}
