//! Ported from `packages/engine/Source/Scene/Model/NodeStatisticsPipelineStage.js`.

/// Pipeline stage for node statistics.
///
/// Collects per-node rendering statistics for debugging/profiling.
pub struct NodeStatisticsPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl NodeStatisticsPipelineStage {
    /// Creates a new NodeStatisticsPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for NodeStatisticsPipelineStage {
    fn default() -> Self { Self::new() }
}
