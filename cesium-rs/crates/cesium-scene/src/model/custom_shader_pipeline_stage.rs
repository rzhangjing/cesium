//! Ported from `packages/engine/Source/Scene/Model/CustomShaderPipelineStage.js`.

/// Pipeline stage for custom shaders.
pub struct CustomShaderPipelineStage {
    _private: (),
}

impl CustomShaderPipelineStage {
    /// Creates a new CustomShaderPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CustomShaderPipelineStage {
    fn default() -> Self { Self::new() }
}
