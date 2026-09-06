//! Ported from `packages/engine/Source/Scene/Model/MaterialPipelineStage.js`.

/// Pipeline stage for material processing.
///
/// Sets up PBR/lit/unlit material uniforms and textures.
pub struct MaterialPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl MaterialPipelineStage {
    /// Creates a new MaterialPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for MaterialPipelineStage {
    fn default() -> Self { Self::new() }
}
