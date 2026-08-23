//! Ported from `packages/engine/Source/Scene/Model/VerticalExaggerationPipelineStage.js`.

/// Pipeline stage for vertical exaggeration.
pub struct VerticalExaggerationPipelineStage {
    _private: (),
}

impl VerticalExaggerationPipelineStage {
    /// Creates a new VerticalExaggerationPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VerticalExaggerationPipelineStage {
    fn default() -> Self { Self::new() }
}
