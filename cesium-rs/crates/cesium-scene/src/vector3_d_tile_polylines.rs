//! Ported from `packages/engine/Source/Scene/Vector3DTilePolylines.js`.

/// Polylines within a vector 3D tile.
///
/// Manages polyline features and their rendering state.
pub struct Vector3DTilePolylines {
    /// The number of polylines.
    pub polylines_length: u32,
    /// Whether the polylines are ready for rendering.
    pub ready: bool,
}

impl Vector3DTilePolylines {
    /// Creates a new Vector3DTilePolylines.
    pub fn new() -> Self { Self { polylines_length: 0, ready: false } }
}

impl Default for Vector3DTilePolylines {
    fn default() -> Self { Self::new() }
}
