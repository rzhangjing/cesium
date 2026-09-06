//! Ported from `packages/engine/Source/Scene/Model/ClassificationModelDrawCommand.js`.

/// Classification model draw command.
///
/// Draw command for classification model rendering.
pub struct ClassificationModelDrawCommand {
    /// Whether the command is active.
    pub active: bool,
}

impl ClassificationModelDrawCommand {
    /// Creates a new ClassificationModelDrawCommand.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for ClassificationModelDrawCommand {
    fn default() -> Self { Self::new() }
}
