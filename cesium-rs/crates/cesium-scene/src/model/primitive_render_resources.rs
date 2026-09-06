//! Ported from `packages/engine/Source/Scene/Model/PrimitiveRenderResources.js`.

/// Primitive render resources.
///
/// Manages GPU resources for a model primitive.
pub struct PrimitiveRenderResources {
    /// Whether resources are allocated.
    pub allocated: bool,
}

impl PrimitiveRenderResources {
    /// Creates a new PrimitiveRenderResources.
    pub fn new() -> Self { Self { allocated: false } }
}

impl Default for PrimitiveRenderResources {
    fn default() -> Self { Self::new() }
}
