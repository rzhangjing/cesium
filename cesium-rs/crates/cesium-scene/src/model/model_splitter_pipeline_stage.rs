//! Ported from `packages/engine/Source/Scene/Model/ModelSplitterPipelineStage.js`.

/// Pipeline stage for model splitting.
pub struct ModelSplitterPipelineStage {
    _private: (),
}

impl ModelSplitterPipelineStage {
    /// Creates a new ModelSplitterPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelSplitterPipelineStage {
    fn default() -> Self { Self::new() }
}
