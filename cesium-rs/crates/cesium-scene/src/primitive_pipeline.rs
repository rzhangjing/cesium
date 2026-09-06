//! Ported from `packages/engine/Source/Scene/PrimitivePipeline.js`.

/// Primitive pipeline.
///
/// Processes geometry primitives for GPU rendering.
pub struct PrimitivePipeline {
    /// Whether the pipeline is active.
    pub active: bool,
}

impl PrimitivePipeline {
    /// Creates a new PrimitivePipeline.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for PrimitivePipeline {
    fn default() -> Self { Self::new() }
}
