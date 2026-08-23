//! Ported from `packages/engine/Source/Scene/BufferPolyline.js`.

/// A polyline in a buffer polyline collection.
pub struct BufferPolyline {
    _private: (),
}

impl BufferPolyline {
    /// Creates a new BufferPolyline.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPolyline {
    fn default() -> Self { Self::new() }
}
