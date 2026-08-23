//! Ported from `packages/engine/Source/Scene/Model/FeatureIdPipelineStage.js`.

/// Pipeline stage for feature ID processing.
pub struct FeatureIdPipelineStage {
    _private: (),
}

impl FeatureIdPipelineStage {
    /// Creates a new FeatureIdPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for FeatureIdPipelineStage {
    fn default() -> Self { Self::new() }
}
