//! Ported from `packages/engine/Source/Scene/BufferPolygon.js`.

/// A polygon in a buffer polygon collection.
///
/// Represents a single polygon with positions and holes.
pub struct BufferPolygon {
    /// The polygon positions as flat coordinate arrays.
    pub positions: Vec<Vec<(f64, f64)>>,
    /// Whether this polygon is visible.
    pub show: bool,
}

impl BufferPolygon {
    /// Creates a new BufferPolygon.
    pub fn new() -> Self {
        Self { positions: Vec::new(), show: true }
    }
}

impl Default for BufferPolygon {
    fn default() -> Self { Self::new() }
}
