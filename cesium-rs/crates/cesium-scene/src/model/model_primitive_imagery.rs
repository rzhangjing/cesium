//! Ported from `packages/engine/Source/Scene/Model/ModelPrimitiveImagery.js`.

/// Model primitive imagery.
///
/// Manages imagery textures on a model primitive.
pub struct ModelPrimitiveImagery {
    /// Whether imagery is active.
    pub active: bool,
}

impl ModelPrimitiveImagery {
    /// Creates a new ModelPrimitiveImagery.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for ModelPrimitiveImagery {
    fn default() -> Self { Self::new() }
}
