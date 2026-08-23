//! Ported from `packages/engine/Source/Scene/Model/DequantizationPipelineStage.js`.

/// Pipeline stage for dequantization.
pub struct DequantizationPipelineStage {
    _private: (),
}

impl DequantizationPipelineStage {
    /// Creates a new DequantizationPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for DequantizationPipelineStage {
    fn default() -> Self { Self::new() }
}
