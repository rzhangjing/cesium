//! Ported from `packages/engine/Source/Scene/Model/ModelSilhouettePipelineStage.js`.

/// Pipeline stage for model silhouette.
pub struct ModelSilhouettePipelineStage {
    _private: (),
}

impl ModelSilhouettePipelineStage {
    /// Creates a new ModelSilhouettePipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelSilhouettePipelineStage {
    fn default() -> Self { Self::new() }
}
