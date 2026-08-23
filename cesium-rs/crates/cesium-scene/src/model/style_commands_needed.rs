//! Ported from `packages/engine/Source/Scene/Model/StyleCommandsNeeded.js`.

/// Tracks which style commands are needed.
pub struct StyleCommandsNeeded {
    _private: (),
}

impl StyleCommandsNeeded {
    /// Creates a new StyleCommandsNeeded.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for StyleCommandsNeeded {
    fn default() -> Self { Self::new() }
}
