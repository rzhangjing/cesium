//! Ported from `packages/engine/Source/Scene/Model/ModelMatrixUpdateStage.js`.

/// Pipeline stage for model matrix updates.
pub struct ModelMatrixUpdateStage {
    _private: (),
}

impl ModelMatrixUpdateStage {
    /// Creates a new ModelMatrixUpdateStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelMatrixUpdateStage {
    fn default() -> Self { Self::new() }
}
