//! Ported from `packages/engine/Source/Scene/Vector3DTileGeometry.js`.

/// Geometry in a vector 3D tile.
pub struct Vector3DTileGeometry {
    _private: (),
}

impl Vector3DTileGeometry {
    /// Creates a new Vector3DTileGeometry.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTileGeometry {
    fn default() -> Self { Self::new() }
}
