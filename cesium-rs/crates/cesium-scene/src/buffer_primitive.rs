//! Ported from `packages/engine/Source/Scene/BufferPrimitive.js`.

/// A primitive in a buffer primitive collection.
///
/// Represents a single geometry primitive with vertex data.
pub struct BufferPrimitive {
    /// Whether this primitive is visible.
    pub show: bool,
    /// The number of vertices.
    pub vertex_count: u32,
    /// The number of indices.
    pub index_count: u32,
}

impl BufferPrimitive {
    /// Creates a new BufferPrimitive.
    pub fn new() -> Self {
        Self { show: true, vertex_count: 0, index_count: 0 }
    }
}

impl Default for BufferPrimitive {
    fn default() -> Self { Self::new() }
}
