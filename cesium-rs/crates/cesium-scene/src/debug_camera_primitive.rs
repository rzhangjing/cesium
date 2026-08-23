//! Ported from `packages/engine/Source/Scene/DebugCameraPrimitive.js`.

/// A debug camera primitive.
pub struct DebugCameraPrimitive {
    _private: (),
}

impl DebugCameraPrimitive {
    /// Creates a new DebugCameraPrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for DebugCameraPrimitive {
    fn default() -> Self { Self::new() }
}
