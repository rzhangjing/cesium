//! Ported from `packages/engine/Source/Scene/TileCoordinatesImageryProvider.js`.

/// An imagery provider that renders tile coordinates.
pub struct TileCoordinatesImageryProvider {
    _private: (),
}

impl TileCoordinatesImageryProvider {
    /// Creates a new TileCoordinatesImageryProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileCoordinatesImageryProvider {
    fn default() -> Self { Self::new() }
}
