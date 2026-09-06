//! Ported from `packages/engine/Source/Scene/Model/SelectedFeatureIdPipelineStage.js`.

/// Pipeline stage for selected feature ID processing.
///
/// Highlights the currently selected feature by ID.
pub struct SelectedFeatureIdPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl SelectedFeatureIdPipelineStage {
    /// Creates a new SelectedFeatureIdPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for SelectedFeatureIdPipelineStage {
    fn default() -> Self { Self::new() }
}
