//! Ported from `packages/engine/Source/Scene/Model/ModelMatrixUpdateStage.js`.

/// Model matrix update stage.
///
/// Updates model matrices each frame.
pub struct ModelMatrixUpdateStage {
    /// Number of updates processed.
    pub update_count: u64,
}

impl ModelMatrixUpdateStage {
    /// Creates a new ModelMatrixUpdateStage.
    pub fn new() -> Self { Self { update_count: 0 } }
}

impl Default for ModelMatrixUpdateStage {
    fn default() -> Self { Self::new() }
}
