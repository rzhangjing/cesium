//! Ported from `packages/engine/Source/Scene/BufferPrimitive.js`.

/// A primitive in a buffer primitive collection.
pub struct BufferPrimitive {
    _private: (),
}

impl BufferPrimitive {
    /// Creates a new BufferPrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPrimitive {
    fn default() -> Self { Self::new() }
}
