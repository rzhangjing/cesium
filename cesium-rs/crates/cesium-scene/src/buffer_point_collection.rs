//! Ported from `packages/engine/Source/Scene/BufferPointCollection.js`.

/// A collection of buffer points.
pub struct BufferPointCollection {
    _private: (),
}

impl BufferPointCollection {
    /// Creates a new BufferPointCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPointCollection {
    fn default() -> Self { Self::new() }
}
