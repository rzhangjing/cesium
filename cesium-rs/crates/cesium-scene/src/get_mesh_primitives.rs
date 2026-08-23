//! Ported from `packages/engine/Source/Scene/getMeshPrimitives.js`.

/// Gets mesh primitives.
pub struct GetMeshPrimitives {
    _private: (),
}

impl GetMeshPrimitives {
    /// Creates a new GetMeshPrimitives.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetMeshPrimitives {
    fn default() -> Self { Self::new() }
}
