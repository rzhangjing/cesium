//! Ported from `packages/engine/Source/Scene/Model/ModelSilhouettePipelineStage.js`.

/// Pipeline stage for model silhouette.
///
/// Renders silhouette/outline around selected or highlighted models.
pub struct ModelSilhouettePipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ModelSilhouettePipelineStage {
    /// Creates a new ModelSilhouettePipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ModelSilhouettePipelineStage {
    fn default() -> Self { Self::new() }
}
