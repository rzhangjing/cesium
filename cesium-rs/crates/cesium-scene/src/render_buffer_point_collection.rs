//! Ported from `packages/engine/Source/Scene/RenderBufferPointCollection.js`.

/// Render buffer point collection.
///
/// GPU-rendered collection of buffer points.
pub struct RenderBufferPointCollection {
    /// Whether the collection is visible.
    pub show: bool,
    /// Whether the collection needs update.
    pub needs_update: bool,
}

impl RenderBufferPointCollection {
    /// Creates a new RenderBufferPointCollection.
    pub fn new() -> Self { Self { show: true, needs_update: false } }
}

impl Default for RenderBufferPointCollection {
    fn default() -> Self { Self::new() }
}
