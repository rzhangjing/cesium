//! Ported from `packages/engine/Source/Scene/Model/VerticalExaggerationPipelineStage.js`.

/// Pipeline stage for vertical exaggeration.
///
/// Applies vertical scale factor to model positions for terrain exaggeration.
pub struct VerticalExaggerationPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl VerticalExaggerationPipelineStage {
    /// Creates a new VerticalExaggerationPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for VerticalExaggerationPipelineStage {
    fn default() -> Self { Self::new() }
}
