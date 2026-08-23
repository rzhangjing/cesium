//! Ported from `packages/engine/Source/Scene/Vector3DTilePolylines.js`.

/// Polylines in a vector 3D tile.
pub struct Vector3DTilePolylines {
    _private: (),
}

impl Vector3DTilePolylines {
    /// Creates a new Vector3DTilePolylines.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTilePolylines {
    fn default() -> Self { Self::new() }
}
