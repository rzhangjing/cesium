//! Ported from `packages/engine/Source/Scene/Model/PrimitiveOutlinePipelineStage.js`.

/// Pipeline stage for primitive outline.
///
/// Renders outlines around selected primitives.
pub struct PrimitiveOutlinePipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl PrimitiveOutlinePipelineStage {
    /// Creates a new PrimitiveOutlinePipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for PrimitiveOutlinePipelineStage {
    fn default() -> Self { Self::new() }
}
