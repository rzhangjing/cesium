//! Ported from `packages/engine/Source/Scene/Model/ModelRuntimeNode.js`.

/// A runtime node in a model.
pub struct ModelRuntimeNode {
    _private: (),
}

impl ModelRuntimeNode {
    /// Creates a new ModelRuntimeNode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelRuntimeNode {
    fn default() -> Self { Self::new() }
}
