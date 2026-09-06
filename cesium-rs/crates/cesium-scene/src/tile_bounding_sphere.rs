//! Ported from `packages/engine/Source/Scene/TileBoundingSphere.js`.

/// Tile bounding sphere.
///
/// Sphere bounding volume for a 3D tile.
pub struct TileBoundingSphere {
    /// The center x coordinate.
    pub center_x: f64,
    /// The center y coordinate.
    pub center_y: f64,
    /// The center z coordinate.
    pub center_z: f64,
    /// The radius.
    pub radius: f64,
}

impl TileBoundingSphere {
    /// Creates a new TileBoundingSphere.
    pub fn new() -> Self { Self { center_x: 0.0, center_y: 0.0, center_z: 0.0, radius: 0.0 } }
}

impl Default for TileBoundingSphere {
    fn default() -> Self { Self::new() }
}
