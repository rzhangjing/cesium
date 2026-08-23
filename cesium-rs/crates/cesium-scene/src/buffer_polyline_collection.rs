//! Ported from `packages/engine/Source/Scene/BufferPolylineCollection.js`.

/// A collection of buffer polylines.
pub struct BufferPolylineCollection {
    _private: (),
}

impl BufferPolylineCollection {
    /// Creates a new BufferPolylineCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BufferPolylineCollection {
    fn default() -> Self { Self::new() }
}
