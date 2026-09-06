//! Ported from `packages/engine/Source/Scene/Model/ModelDrawCommands.js`.

/// Model draw commands.
///
/// Collection of draw commands for model rendering.
pub struct ModelDrawCommands {
    /// The number of draw commands.
    pub length: u32,
}

impl ModelDrawCommands {
    /// Creates a new ModelDrawCommands.
    pub fn new() -> Self { Self { length: 0 } }
}

impl Default for ModelDrawCommands {
    fn default() -> Self { Self::new() }
}
