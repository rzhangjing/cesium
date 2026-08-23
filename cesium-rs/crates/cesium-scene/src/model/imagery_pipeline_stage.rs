//! Ported from `packages/engine/Source/Scene/Model/ImageryPipelineStage.js`.

/// Pipeline stage for imagery processing.
pub struct ImageryPipelineStage {
    _private: (),
}

impl ImageryPipelineStage {
    /// Creates a new ImageryPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageryPipelineStage {
    fn default() -> Self { Self::new() }
}
