//! Ported from `packages/engine/Source/Scene/TileBoundingS2Cell.js`.

/// Tile bounding S2 cell.
///
/// S2 cell bounding volume for a 3D tile.
pub struct TileBoundingS2Cell {
    /// The S2 cell token.
    pub token: String,
    /// The minimum height.
    pub minimum_height: f64,
    /// The maximum height.
    pub maximum_height: f64,
}

impl TileBoundingS2Cell {
    /// Creates a new TileBoundingS2Cell.
    pub fn new() -> Self { Self { token: String::new(), minimum_height: 0.0, maximum_height: 0.0 } }
}

impl Default for TileBoundingS2Cell {
    fn default() -> Self { Self::new() }
}
