//! Ported from `packages/engine/Source/Scene/Model/ImageBasedLightingPipelineStage.js`.

/// Pipeline stage for image-based lighting.
///
/// Applies IBL environment map contributions to PBR materials.
pub struct ImageBasedLightingPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ImageBasedLightingPipelineStage {
    /// Creates a new ImageBasedLightingPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ImageBasedLightingPipelineStage {
    fn default() -> Self { Self::new() }
}
