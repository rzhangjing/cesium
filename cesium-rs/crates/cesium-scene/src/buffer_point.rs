//! Ported from `packages/engine/Source/Scene/BufferPoint.js`.

/// A point in a buffer point collection.
pub struct BufferPoint {
    _private: (),
}

impl BufferPoint {
    /// Creates a new BufferPoint.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPoint {
    fn default() -> Self { Self::new() }
}
