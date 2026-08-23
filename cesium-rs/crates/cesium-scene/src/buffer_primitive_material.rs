//! Ported from `packages/engine/Source/Scene/BufferPrimitiveMaterial.js`.

/// A material for buffer primitives.
pub struct BufferPrimitiveMaterial {
    _private: (),
}

impl BufferPrimitiveMaterial {
    /// Creates a new BufferPrimitiveMaterial.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPrimitiveMaterial {
    fn default() -> Self { Self::new() }
}
