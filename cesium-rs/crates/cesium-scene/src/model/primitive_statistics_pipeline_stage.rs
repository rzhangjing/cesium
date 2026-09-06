//! Ported from `packages/engine/Source/Scene/Model/PrimitiveStatisticsPipelineStage.js`.

/// Pipeline stage for primitive statistics.
///
/// Collects per-primitive rendering statistics for debugging/profiling.
pub struct PrimitiveStatisticsPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl PrimitiveStatisticsPipelineStage {
    /// Creates a new PrimitiveStatisticsPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for PrimitiveStatisticsPipelineStage {
    fn default() -> Self { Self::new() }
}
