//! Ported from `packages/engine/Source/Scene/Model/MorphTargetsPipelineStage.js`.

/// Pipeline stage for morph targets.
///
/// Applies glTF morph target (shape key) animations to vertex data.
pub struct MorphTargetsPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl MorphTargetsPipelineStage {
    /// Creates a new MorphTargetsPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for MorphTargetsPipelineStage {
    fn default() -> Self { Self::new() }
}
