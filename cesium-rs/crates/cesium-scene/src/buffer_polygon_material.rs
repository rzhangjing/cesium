//! Ported from `packages/engine/Source/Scene/BufferPolygonMaterial.js`.

/// Material for buffer polygons.
///
/// Defines the appearance of polygons in a buffer polygon collection.
pub struct BufferPolygonMaterial {
    /// Whether the material is transparent.
    pub transparent: bool,
}

impl BufferPolygonMaterial {
    /// Creates a new BufferPolygonMaterial.
    pub fn new() -> Self { Self { transparent: false } }
}

impl Default for BufferPolygonMaterial {
    fn default() -> Self { Self::new() }
}
