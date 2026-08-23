//! Ported from `packages/engine/Source/Scene/BufferPrimitiveCollection.js`.

/// A collection of buffer primitives.
pub struct BufferPrimitiveCollection {
    _private: (),
}

impl BufferPrimitiveCollection {
    /// Creates a new BufferPrimitiveCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPrimitiveCollection {
    fn default() -> Self { Self::new() }
}
