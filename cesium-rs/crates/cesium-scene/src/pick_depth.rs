//! Ported from `packages/engine/Source/Scene/PickDepth.js`.

/// Depth buffer for picking.
pub struct PickDepth {
    _private: (),
}

impl PickDepth {
    /// Creates a new PickDepth.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PickDepth {
    fn default() -> Self { Self::new() }
}
