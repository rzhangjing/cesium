//! Ported from `packages/engine/Source/Scene/DebugAppearance.js`.

/// A debug appearance.
pub struct DebugAppearance {
    _private: (),
}

impl DebugAppearance {
    /// Creates a new DebugAppearance.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for DebugAppearance {
    fn default() -> Self { Self::new() }
}
