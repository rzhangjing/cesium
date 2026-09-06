//! Ported from `packages/engine/Source/Scene/Model/ModelSplitterPipelineStage.js`.

/// Pipeline stage for model splitter.
///
/// Applies left/right split rendering for A/B comparison.
pub struct ModelSplitterPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ModelSplitterPipelineStage {
    /// Creates a new ModelSplitterPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ModelSplitterPipelineStage {
    fn default() -> Self { Self::new() }
}
