//! Ported from `packages/engine/Source/Scene/Model/SkinningPipelineStage.js`.

/// Pipeline stage for skeletal skinning.
pub struct SkinningPipelineStage {
    _private: (),
}

impl SkinningPipelineStage {
    /// Creates a new SkinningPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SkinningPipelineStage {
    fn default() -> Self { Self::new() }
}
