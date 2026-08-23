//! Ported from `packages/engine/Source/Scene/Vector3DTilePoints.js`.

/// Points in a vector 3D tile.
pub struct Vector3DTilePoints {
    _private: (),
}

impl Vector3DTilePoints {
    /// Creates a new Vector3DTilePoints.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTilePoints {
    fn default() -> Self { Self::new() }
}
