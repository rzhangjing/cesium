//! Ported from `packages/engine/Source/Scene/RenderBufferPolygonCollection.js`.

/// Render buffer polygon collection.
///
/// GPU-rendered collection of buffer polygons.
pub struct RenderBufferPolygonCollection {
    /// Whether the collection is visible.
    pub show: bool,
    /// Whether the collection needs update.
    pub needs_update: bool,
}

impl RenderBufferPolygonCollection {
    /// Creates a new RenderBufferPolygonCollection.
    pub fn new() -> Self { Self { show: true, needs_update: false } }
}

impl Default for RenderBufferPolygonCollection {
    fn default() -> Self { Self::new() }
}
