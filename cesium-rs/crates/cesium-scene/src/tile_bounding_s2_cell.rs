//! Ported from `packages/engine/Source/Scene/TileBoundingS2Cell.js`.

/// An S2 cell bounding volume for a tile.
pub struct TileBoundingS2Cell {
    _private: (),
}

impl TileBoundingS2Cell {
    /// Creates a new TileBoundingS2Cell.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TileBoundingS2Cell {
    fn default() -> Self { Self::new() }
}
