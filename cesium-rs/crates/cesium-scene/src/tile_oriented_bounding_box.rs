//! Ported from `packages/engine/Source/Scene/TileOrientedBoundingBox.js`.

/// Tile oriented bounding box.
///
/// Oriented bounding box volume for a 3D tile.
pub struct TileOrientedBoundingBox {
    /// The half-axes lengths.
    pub half_axes: (f64, f64, f64),
    /// The center position.
    pub center: (f64, f64, f64),
}

impl TileOrientedBoundingBox {
    /// Creates a new TileOrientedBoundingBox.
    pub fn new() -> Self { Self { half_axes: (0.0, 0.0, 0.0), center: (0.0, 0.0, 0.0) } }
}

impl Default for TileOrientedBoundingBox {
    fn default() -> Self { Self::new() }
}
