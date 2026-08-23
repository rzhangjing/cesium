//! Ported from `packages/engine/Source/Scene/Model/MorphTargetsPipelineStage.js`.

/// Pipeline stage for morph targets.
pub struct MorphTargetsPipelineStage {
    _private: (),
}

impl MorphTargetsPipelineStage {
    /// Creates a new MorphTargetsPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MorphTargetsPipelineStage {
    fn default() -> Self { Self::new() }
}
