//! Ported from `packages/engine/Source/Scene/BufferPolygonMaterial.js`.

/// A material for buffer polygons.
pub struct BufferPolygonMaterial {
    _private: (),
}

impl BufferPolygonMaterial {
    /// Creates a new BufferPolygonMaterial.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPolygonMaterial {
    fn default() -> Self { Self::new() }
}
