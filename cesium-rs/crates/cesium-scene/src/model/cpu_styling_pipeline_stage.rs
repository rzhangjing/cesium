//! Ported from `packages/engine/Source/Scene/Model/CpuStylingPipelineStage.js`.

/// Pipeline stage for CPU-side styling.
///
/// Applies per-feature color/style on the CPU before GPU submission.
pub struct CpuStylingPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl CpuStylingPipelineStage {
    /// Creates a new CpuStylingPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for CpuStylingPipelineStage {
    fn default() -> Self { Self::new() }
}
