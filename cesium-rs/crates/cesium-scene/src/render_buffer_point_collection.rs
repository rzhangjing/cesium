//! Ported from `packages/engine/Source/Scene/RenderBufferPointCollection.js`.

/// A render buffer for point collections.
pub struct RenderBufferPointCollection {
    _private: (),
}

impl RenderBufferPointCollection {
    /// Creates a new RenderBufferPointCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for RenderBufferPointCollection {
    fn default() -> Self { Self::new() }
}
