//! Ported from `packages/engine/Source/Scene/Vector3DTileClampedPolylines.js`.

/// Clamped polylines in a vector 3D tile.
pub struct Vector3DTileClampedPolylines {
    _private: (),
}

impl Vector3DTileClampedPolylines {
    /// Creates a new Vector3DTileClampedPolylines.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTileClampedPolylines {
    fn default() -> Self { Self::new() }
}
