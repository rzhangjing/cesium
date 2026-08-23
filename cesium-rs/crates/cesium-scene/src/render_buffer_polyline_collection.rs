//! Ported from `packages/engine/Source/Scene/RenderBufferPolylineCollection.js`.

/// A render buffer for polyline collections.
pub struct RenderBufferPolylineCollection {
    _private: (),
}

impl RenderBufferPolylineCollection {
    /// Creates a new RenderBufferPolylineCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for RenderBufferPolylineCollection {
    fn default() -> Self { Self::new() }
}
