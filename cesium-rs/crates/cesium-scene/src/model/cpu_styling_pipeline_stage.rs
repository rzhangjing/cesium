//! Ported from `packages/engine/Source/Scene/Model/CpuStylingPipelineStage.js`.

/// Pipeline stage for CPU-side styling.
pub struct CpuStylingPipelineStage {
    _private: (),
}

impl CpuStylingPipelineStage {
    /// Creates a new CpuStylingPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CpuStylingPipelineStage {
    fn default() -> Self { Self::new() }
}
