//! Ported from `packages/engine/Source/Scene/Vector3DTilePolygons.js`.

/// Polygons within a vector 3D tile.
///
/// Manages polygon features and their rendering state.
pub struct Vector3DTilePolygons {
    /// The number of polygons.
    pub polygons_length: u32,
    /// Whether the polygons are ready for rendering.
    pub ready: bool,
}

impl Vector3DTilePolygons {
    /// Creates a new Vector3DTilePolygons.
    pub fn new() -> Self { Self { polygons_length: 0, ready: false } }
}

impl Default for Vector3DTilePolygons {
    fn default() -> Self { Self::new() }
}
