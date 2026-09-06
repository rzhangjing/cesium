//! Ported from `packages/engine/Source/Scene/BufferPolylineMaterial.js`.

/// Material for buffer polylines.
///
/// Defines the appearance of polylines in a buffer polyline collection.
pub struct BufferPolylineMaterial {
    /// Whether the material is transparent.
    pub transparent: bool,
}

impl BufferPolylineMaterial {
    /// Creates a new BufferPolylineMaterial.
    pub fn new() -> Self { Self { transparent: false } }
}

impl Default for BufferPolylineMaterial {
    fn default() -> Self { Self::new() }
}
