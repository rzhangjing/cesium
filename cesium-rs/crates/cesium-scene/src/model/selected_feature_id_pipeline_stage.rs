//! Ported from `packages/engine/Source/Scene/Model/SelectedFeatureIdPipelineStage.js`.

/// Pipeline stage for selected feature IDs.
pub struct SelectedFeatureIdPipelineStage {
    _private: (),
}

impl SelectedFeatureIdPipelineStage {
    /// Creates a new SelectedFeatureIdPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SelectedFeatureIdPipelineStage {
    fn default() -> Self { Self::new() }
}
