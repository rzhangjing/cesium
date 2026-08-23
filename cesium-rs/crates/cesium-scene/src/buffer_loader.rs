//! Ported from `packages/engine/Source/Scene/BufferLoader.js`.

/// Loads GPU buffers from data.
pub struct BufferLoader {
    _private: (),
}

impl BufferLoader {
    /// Creates a new BufferLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferLoader {
    fn default() -> Self { Self::new() }
}
