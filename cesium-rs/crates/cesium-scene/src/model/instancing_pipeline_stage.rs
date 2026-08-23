//! Ported from `packages/engine/Source/Scene/Model/InstancingPipelineStage.js`.

/// Pipeline stage for instancing.
pub struct InstancingPipelineStage {
    _private: (),
}

impl InstancingPipelineStage {
    /// Creates a new InstancingPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for InstancingPipelineStage {
    fn default() -> Self { Self::new() }
}
