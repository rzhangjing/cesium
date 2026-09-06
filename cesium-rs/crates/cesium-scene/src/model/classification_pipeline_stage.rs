//! Ported from `packages/engine/Source/Scene/Model/ClassificationPipelineStage.js`.

/// Pipeline stage for classification processing.
///
/// Configures render commands for classification (terrain/3D Tiles overlay).
pub struct ClassificationPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ClassificationPipelineStage {
    /// Creates a new ClassificationPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ClassificationPipelineStage {
    fn default() -> Self { Self::new() }
}
