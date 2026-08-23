//! Ported from `packages/engine/Source/Scene/Model/ModelDrawCommands.js`.

/// Collection of draw commands for a model.
pub struct ModelDrawCommands {
    _private: (),
}

impl ModelDrawCommands {
    /// Creates a new ModelDrawCommands.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelDrawCommands {
    fn default() -> Self { Self::new() }
}
