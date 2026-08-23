//! Ported from `packages/engine/Source/Scene/Model/ModelDrawCommand.js`.

/// A draw command for a model.
pub struct ModelDrawCommand {
    _private: (),
}

impl ModelDrawCommand {
    /// Creates a new ModelDrawCommand.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelDrawCommand {
    fn default() -> Self { Self::new() }
}
