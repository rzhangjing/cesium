//! Ported from `packages/engine/Source/Scene/BufferPointMaterial.js`.

/// A material for buffer points.
pub struct BufferPointMaterial {
    _private: (),
}

impl BufferPointMaterial {
    /// Creates a new BufferPointMaterial.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPointMaterial {
    fn default() -> Self { Self::new() }
}
