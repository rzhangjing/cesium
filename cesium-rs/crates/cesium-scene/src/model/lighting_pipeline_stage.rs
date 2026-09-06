//! Ported from `packages/engine/Source/Scene/Model/LightingPipelineStage.js`.

/// Pipeline stage for lighting.
///
/// Applies directional and ambient lighting calculations to model shaders.
pub struct LightingPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl LightingPipelineStage {
    /// Creates a new LightingPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for LightingPipelineStage {
    fn default() -> Self { Self::new() }
}
