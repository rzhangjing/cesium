//! Ported from `packages/engine/Source/Scene/DerivedCommand.js`.

/// A derived command.
///
/// A command derived from an original draw command (e.g. for picking or shadows).
pub struct DerivedCommand {
    /// Whether the derived command is ready.
    pub ready: bool,
}

impl DerivedCommand {
    /// Creates a new DerivedCommand.
    pub fn new() -> Self { Self { ready: false } }
}

impl Default for DerivedCommand {
    fn default() -> Self { Self::new() }
}
