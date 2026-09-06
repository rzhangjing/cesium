//! Ported from `packages/engine/Source/Scene/DebugInspector.js`.

/// Debug inspector.
///
/// Provides debugging visualization overlays for scene inspection.
pub struct DebugInspector {
    /// Whether the inspector is visible.
    pub show: bool,
}

impl DebugInspector {
    /// Creates a new DebugInspector.
    pub fn new() -> Self { Self { show: false } }
}

impl Default for DebugInspector {
    fn default() -> Self { Self::new() }
}
