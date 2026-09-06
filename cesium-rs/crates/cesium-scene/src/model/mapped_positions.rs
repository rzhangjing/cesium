//! Ported from `packages/engine/Source/Scene/Model/MappedPositions.js`.

use serde_json::Value;

/// A collection of cartographic positions (and their bounding rectangle)
/// that have been computed from cartesian positions, for a specific ellipsoid.
#[derive(Debug, Clone)]
pub struct MappedPositions {
    /// The cartographic positions.
    pub cartographic_positions: Value,
    /// The number of positions.
    pub num_positions: usize,
    /// The bounding rectangle of the positions.
    pub cartographic_bounding_rectangle: Value,
    /// The ellipsoid for which these positions were created.
    pub ellipsoid: Value,
}

impl MappedPositions {
    /// Creates a new `MappedPositions`.
    pub fn new(
        cartographic_positions: Value,
        num_positions: usize,
        cartographic_bounding_rectangle: Value,
        ellipsoid: Value,
    ) -> Self {
        Self {
            cartographic_positions,
            num_positions,
            cartographic_bounding_rectangle,
            ellipsoid,
        }
    }
}
