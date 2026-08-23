//! Ported from `packages/engine/Source/Scene/Model/ImageBasedLightingPipelineStage.js`.

/// Pipeline stage for image-based lighting.
pub struct ImageBasedLightingPipelineStage {
    _private: (),
}

impl ImageBasedLightingPipelineStage {
    /// Creates a new ImageBasedLightingPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageBasedLightingPipelineStage {
    fn default() -> Self { Self::new() }
}
