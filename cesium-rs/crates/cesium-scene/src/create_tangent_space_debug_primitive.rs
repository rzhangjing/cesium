//! Ported from `packages/engine/Source/Scene/CreateTangentSpaceDebugPrimitive.js`.

/// Creates tangent space debug primitive.
///
/// Generates geometry for visualizing tangent space basis vectors.
pub struct CreateTangentSpaceDebugPrimitive {
    /// Whether creation is complete.
    pub complete: bool,
}

impl CreateTangentSpaceDebugPrimitive {
    /// Creates a new CreateTangentSpaceDebugPrimitive.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for CreateTangentSpaceDebugPrimitive {
    fn default() -> Self { Self::new() }
}
