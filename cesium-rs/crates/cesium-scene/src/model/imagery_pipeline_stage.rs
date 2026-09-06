//! Ported from `packages/engine/Source/Scene/Model/ImageryPipelineStage.js`.

/// Pipeline stage for imagery processing.
///
/// Applies imagery layer textures to model surfaces.
pub struct ImageryPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl ImageryPipelineStage {
    /// Creates a new ImageryPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for ImageryPipelineStage {
    fn default() -> Self { Self::new() }
}
