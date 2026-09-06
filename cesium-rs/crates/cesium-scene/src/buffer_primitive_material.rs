//! Ported from `packages/engine/Source/Scene/BufferPrimitiveMaterial.js`.

/// Material for buffer primitives.
///
/// Defines the appearance of primitives in a buffer primitive collection.
pub struct BufferPrimitiveMaterial {
    /// Whether the material is transparent.
    pub transparent: bool,
}

impl BufferPrimitiveMaterial {
    /// Creates a new BufferPrimitiveMaterial.
    pub fn new() -> Self { Self { transparent: false } }
}

impl Default for BufferPrimitiveMaterial {
    fn default() -> Self { Self::new() }
}
