//! Ported from `packages/engine/Source/Scene/Vector3DTilePoints.js`.

/// Points within a vector 3D tile.
///
/// Manages point features and their rendering state.
pub struct Vector3DTilePoints {
    /// The number of points.
    pub points_length: u32,
    /// Whether the points are ready for rendering.
    pub ready: bool,
}

impl Vector3DTilePoints {
    /// Creates a new Vector3DTilePoints.
    pub fn new() -> Self { Self { points_length: 0, ready: false } }
}

impl Default for Vector3DTilePoints {
    fn default() -> Self { Self::new() }
}
