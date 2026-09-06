//! Ported from `packages/engine/Source/Scene/BufferPointMaterial.js`.

/// Material for buffer points.
///
/// Defines the appearance of points in a buffer point collection.
pub struct BufferPointMaterial {
    /// Whether the material is transparent.
    pub transparent: bool,
}

impl BufferPointMaterial {
    /// Creates a new BufferPointMaterial.
    pub fn new() -> Self { Self { transparent: false } }
}

impl Default for BufferPointMaterial {
    fn default() -> Self { Self::new() }
}
