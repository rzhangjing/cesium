//! Ported from `packages/engine/Source/Scene/Vector3DTilePolygons.js`.

/// Polygons in a vector 3D tile.
pub struct Vector3DTilePolygons {
    _private: (),
}

impl Vector3DTilePolygons {
    /// Creates a new Vector3DTilePolygons.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Vector3DTilePolygons {
    fn default() -> Self { Self::new() }
}
