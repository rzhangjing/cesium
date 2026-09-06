//! Ported from `packages/engine/Source/Scene/Vector3DTileClampedPolylines.js`.

/// Clamped polylines within a vector 3D tile.
///
/// Manages polylines that are clamped to terrain or 3D Tiles surfaces.
pub struct Vector3DTileClampedPolylines {
    /// The number of polylines.
    pub polylines_length: u32,
    /// Whether the data is ready.
    pub ready: bool,
}

impl Vector3DTileClampedPolylines {
    /// Creates a new Vector3DTileClampedPolylines.
    pub fn new() -> Self { Self { polylines_length: 0, ready: false } }
}

impl Default for Vector3DTileClampedPolylines {
    fn default() -> Self { Self::new() }
}
