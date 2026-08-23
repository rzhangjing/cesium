//! Ported from `packages/engine/Source/Scene/createTangentSpaceDebugPrimitive.js`.

/// Creates a tangent space debug primitive.
pub struct CreateTangentSpaceDebugPrimitive {
    _private: (),
}

impl CreateTangentSpaceDebugPrimitive {
    /// Creates a new CreateTangentSpaceDebugPrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateTangentSpaceDebugPrimitive {
    fn default() -> Self { Self::new() }
}
