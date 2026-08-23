//! Ported from `packages/engine/Source/Scene/RenderBufferPolygonCollection.js`.

/// A render buffer for polygon collections.
pub struct RenderBufferPolygonCollection {
    _private: (),
}

impl RenderBufferPolygonCollection {
    /// Creates a new RenderBufferPolygonCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for RenderBufferPolygonCollection {
    fn default() -> Self { Self::new() }
}
