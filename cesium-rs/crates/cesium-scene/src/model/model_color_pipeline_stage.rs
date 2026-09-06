//! Ported from `packages/engine/Source/Scene/Model/ModelColorPipelineStage.js`.

/// Pipeline stage for model color.
///
/// Applies model-level color, silhouette, and highlight effects.
pub struct ModelColorPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ModelColorPipelineStage {
    /// Creates a new ModelColorPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ModelColorPipelineStage {
    fn default() -> Self { Self::new() }
}
