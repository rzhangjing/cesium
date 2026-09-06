//! Ported from `packages/engine/Source/Scene/Model/CustomShaderPipelineStage.js`.

/// Pipeline stage for custom shader processing.
///
/// Applies user-defined CustomShader modifications to model materials.
pub struct CustomShaderPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl CustomShaderPipelineStage {
    /// Creates a new CustomShaderPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for CustomShaderPipelineStage {
    fn default() -> Self { Self::new() }
}
