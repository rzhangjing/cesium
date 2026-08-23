//! Ported from `packages/engine/Source/Scene/Model/ClassificationPipelineStage.js`.

/// Pipeline stage for classification.
pub struct ClassificationPipelineStage {
    _private: (),
}

impl ClassificationPipelineStage {
    /// Creates a new ClassificationPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ClassificationPipelineStage {
    fn default() -> Self { Self::new() }
}
