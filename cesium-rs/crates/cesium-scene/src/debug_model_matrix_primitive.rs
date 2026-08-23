//! Ported from `packages/engine/Source/Scene/DebugModelMatrixPrimitive.js`.

/// A debug model matrix primitive.
pub struct DebugModelMatrixPrimitive {
    _private: (),
}

impl DebugModelMatrixPrimitive {
    /// Creates a new DebugModelMatrixPrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for DebugModelMatrixPrimitive {
    fn default() -> Self { Self::new() }
}
