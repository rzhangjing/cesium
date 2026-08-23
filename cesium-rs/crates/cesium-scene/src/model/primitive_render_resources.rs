//! Ported from `packages/engine/Source/Scene/Model/PrimitiveRenderResources.js`.

/// Rendering resources for a primitive.
pub struct PrimitiveRenderResources {
    _private: (),
}

impl PrimitiveRenderResources {
    /// Creates a new PrimitiveRenderResources.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PrimitiveRenderResources {
    fn default() -> Self { Self::new() }
}
