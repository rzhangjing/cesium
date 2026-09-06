//! Ported from `packages/engine/Source/Scene/Model/AlphaPipelineStage.js`.

/// Pipeline stage for alpha processing.
///
/// Processes alpha/transparency for model render commands.
pub struct AlphaPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl AlphaPipelineStage {
    /// Creates a new AlphaPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for AlphaPipelineStage {
    fn default() -> Self { Self::new() }
}
