//! Ported from `packages/engine/Source/Scene/Model/ModelRuntimePrimitive.js`.

/// A runtime primitive in a model.
pub struct ModelRuntimePrimitive {
    _private: (),
}

impl ModelRuntimePrimitive {
    /// Creates a new ModelRuntimePrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelRuntimePrimitive {
    fn default() -> Self { Self::new() }
}
