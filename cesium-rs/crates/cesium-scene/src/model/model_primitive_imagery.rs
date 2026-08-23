//! Ported from `packages/engine/Source/Scene/Model/ModelPrimitiveImagery.js`.

/// Imagery for a model primitive.
pub struct ModelPrimitiveImagery {
    _private: (),
}

impl ModelPrimitiveImagery {
    /// Creates a new ModelPrimitiveImagery.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelPrimitiveImagery {
    fn default() -> Self { Self::new() }
}
