//! Ported from `packages/engine/Source/Scene/DerivedCommand.js`.

/// A derived command.
///
/// DEVIATION: requires Scene infrastructure for full implementation.
pub struct DerivedCommand {
    _private: (),
}

impl DerivedCommand {
    /// Creates a new derived command.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DerivedCommand {
    fn default() -> Self { Self::new() }
}
