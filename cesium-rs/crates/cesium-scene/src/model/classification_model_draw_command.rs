//! Ported from `packages/engine/Source/Scene/Model/ClassificationModelDrawCommand.js`.

/// A draw command for classification models.
pub struct ClassificationModelDrawCommand {
    _private: (),
}

impl ClassificationModelDrawCommand {
    /// Creates a new ClassificationModelDrawCommand.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ClassificationModelDrawCommand {
    fn default() -> Self { Self::new() }
}
