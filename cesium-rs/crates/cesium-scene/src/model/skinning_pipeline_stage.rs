//! Ported from `packages/engine/Source/Scene/Model/SkinningPipelineStage.js`.

/// Pipeline stage for skeletal skinning.
///
/// Applies bone transforms for skinned mesh animation.
pub struct SkinningPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl SkinningPipelineStage {
    /// Creates a new SkinningPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for SkinningPipelineStage {
    fn default() -> Self { Self::new() }
}
