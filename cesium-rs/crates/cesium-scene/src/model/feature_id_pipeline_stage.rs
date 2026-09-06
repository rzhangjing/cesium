//! Ported from `packages/engine/Source/Scene/Model/FeatureIdPipelineStage.js`.

/// Pipeline stage for feature ID processing.
///
/// Sets up feature ID attributes for per-feature picking and styling.
pub struct FeatureIdPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl FeatureIdPipelineStage {
    /// Creates a new FeatureIdPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for FeatureIdPipelineStage {
    fn default() -> Self { Self::new() }
}
