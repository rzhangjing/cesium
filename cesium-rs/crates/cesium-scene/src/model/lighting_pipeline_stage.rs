//! Ported from `packages/engine/Source/Scene/Model/LightingPipelineStage.js`.

/// Pipeline stage for lighting.
pub struct LightingPipelineStage {
    _private: (),
}

impl LightingPipelineStage {
    /// Creates a new LightingPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for LightingPipelineStage {
    fn default() -> Self { Self::new() }
}
