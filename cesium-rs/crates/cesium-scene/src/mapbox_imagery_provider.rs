//! Ported from `packages/engine/Source/Scene/MapboxImageryProvider.js`.

/// Mapbox imagery provider.
pub struct MapboxImageryProvider {
    _private: (),
}

impl MapboxImageryProvider {
    /// Creates a new MapboxImageryProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MapboxImageryProvider {
    fn default() -> Self { Self::new() }
}
