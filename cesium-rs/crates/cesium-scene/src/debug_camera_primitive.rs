//! Ported from `packages/engine/Source/Scene/DebugCameraPrimitive.js`.

/// Debug camera primitive.
///
/// Renders a wireframe frustum to visualize a camera's view volume.
pub struct DebugCameraPrimitive {
    /// Whether the primitive is visible.
    pub show: bool,
    /// The frustum near distance.
    pub near: f64,
    /// The frustum far distance.
    pub far: f64,
}

impl DebugCameraPrimitive {
    /// Creates a new DebugCameraPrimitive.
    pub fn new() -> Self { Self { show: true, near: 1.0, far: 1000.0 } }
}

impl Default for DebugCameraPrimitive {
    fn default() -> Self { Self::new() }
}
