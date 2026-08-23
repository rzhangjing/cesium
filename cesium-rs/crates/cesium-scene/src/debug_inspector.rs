//! Ported from `packages/engine/Source/Scene/DebugInspector.js`.

/// A debug inspector.
pub struct DebugInspector {
    _private: (),
}

impl DebugInspector {
    /// Creates a new DebugInspector.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for DebugInspector {
    fn default() -> Self { Self::new() }
}
