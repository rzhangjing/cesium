//! Ported from `packages/engine/Source/Scene/BufferPolygon.js`.

/// A polygon in a buffer polygon collection.
pub struct BufferPolygon {
    _private: (),
}

impl BufferPolygon {
    /// Creates a new BufferPolygon.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPolygon {
    fn default() -> Self { Self::new() }
}
