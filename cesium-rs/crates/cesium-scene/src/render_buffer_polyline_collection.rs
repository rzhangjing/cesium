//! Ported from `packages/engine/Source/Scene/RenderBufferPolylineCollection.js`.

/// Render buffer polyline collection.
///
/// GPU-rendered collection of buffer polylines.
pub struct RenderBufferPolylineCollection {
    /// Whether the collection is visible.
    pub show: bool,
    /// Whether the collection needs update.
    pub needs_update: bool,
}

impl RenderBufferPolylineCollection {
    /// Creates a new RenderBufferPolylineCollection.
    pub fn new() -> Self { Self { show: true, needs_update: false } }
}

impl Default for RenderBufferPolylineCollection {
    fn default() -> Self { Self::new() }
}
