//! Ported from `packages/engine/Source/Scene/TileBoundingRegion.js`.

/// A region bounding volume for a tile.
pub struct TileBoundingRegion {
    _private: (),
}

impl TileBoundingRegion {
    /// Creates a new TileBoundingRegion.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileBoundingRegion {
    fn default() -> Self { Self::new() }
}
