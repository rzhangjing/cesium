//! Ported from `packages/engine/Source/Scene/BufferPolylineMaterial.js`.

/// A material for buffer polylines.
pub struct BufferPolylineMaterial {
    _private: (),
}

impl BufferPolylineMaterial {
    /// Creates a new BufferPolylineMaterial.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPolylineMaterial {
    fn default() -> Self { Self::new() }
}
