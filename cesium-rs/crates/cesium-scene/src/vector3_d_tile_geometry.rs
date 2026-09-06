//! Ported from `packages/engine/Source/Scene/Vector3DTileGeometry.js`.

/// Geometry data within a vector 3D tile.
///
/// Contains vertex/index data for vector tile rendering.
pub struct Vector3DTileGeometry {
    /// The number of vertices.
    pub vertex_count: u32,
    /// The number of indices.
    pub index_count: u32,
    /// Whether the geometry is ready.
    pub ready: bool,
}

impl Vector3DTileGeometry {
    /// Creates a new Vector3DTileGeometry.
    pub fn new() -> Self { Self { vertex_count: 0, index_count: 0, ready: false } }
}

impl Default for Vector3DTileGeometry {
    fn default() -> Self { Self::new() }
}
