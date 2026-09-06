//! Ported from `packages/engine/Source/Scene/Model/DequantizationPipelineStage.js`.

/// Pipeline stage for dequantization.
///
/// Expands quantized vertex attributes (e.g. uint16 positions) back to float.
pub struct DequantizationPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl DequantizationPipelineStage {
    /// Creates a new DequantizationPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for DequantizationPipelineStage {
    fn default() -> Self { Self::new() }
}
