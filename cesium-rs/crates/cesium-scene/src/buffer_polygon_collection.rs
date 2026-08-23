//! Ported from `packages/engine/Source/Scene/BufferPolygonCollection.js`.

/// A collection of buffer polygons.
pub struct BufferPolygonCollection {
    _private: (),
}

impl BufferPolygonCollection {
    /// Creates a new BufferPolygonCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPolygonCollection {
    fn default() -> Self { Self::new() }
}
