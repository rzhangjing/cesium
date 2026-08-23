//! Ported from `packages/engine/Source/Scene/Model/NodeStatisticsPipelineStage.js`.

/// Pipeline stage for node statistics.
pub struct NodeStatisticsPipelineStage {
    _private: (),
}

impl NodeStatisticsPipelineStage {
    /// Creates a new NodeStatisticsPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for NodeStatisticsPipelineStage {
    fn default() -> Self { Self::new() }
}
