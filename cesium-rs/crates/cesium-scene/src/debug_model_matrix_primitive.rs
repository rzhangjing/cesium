//! Ported from `packages/engine/Source/Scene/DebugModelMatrixPrimitive.js`.

/// Debug model matrix primitive.
///
/// Renders axes to visualize a model matrix transform.
pub struct DebugModelMatrixPrimitive {
    /// The axis length.
    pub length: f64,
    /// Whether the primitive is visible.
    pub show: bool,
}

impl DebugModelMatrixPrimitive {
    /// Creates a new DebugModelMatrixPrimitive.
    pub fn new() -> Self { Self { length: 1000000.0, show: true } }
}

impl Default for DebugModelMatrixPrimitive {
    fn default() -> Self { Self::new() }
}
