//! Ported from `packages/engine/Source/Scene/Model/PrimitiveStatisticsPipelineStage.js`.

/// Pipeline stage for primitive statistics.
pub struct PrimitiveStatisticsPipelineStage {
    _private: (),
}

impl PrimitiveStatisticsPipelineStage {
    /// Creates a new PrimitiveStatisticsPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PrimitiveStatisticsPipelineStage {
    fn default() -> Self { Self::new() }
}
