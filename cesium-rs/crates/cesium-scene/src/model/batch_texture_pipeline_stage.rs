//! Ported from `packages/engine/Source/Scene/Model/BatchTexturePipelineStage.js`.

/// Pipeline stage for batch texture processing.
///
/// Sets up batch texture uniforms for per-feature picking and styling.
pub struct BatchTexturePipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl BatchTexturePipelineStage {
    /// Creates a new BatchTexturePipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for BatchTexturePipelineStage {
    fn default() -> Self { Self::new() }
}
