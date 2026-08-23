//! Ported from `packages/engine/Source/Scene/PrimitiveLoadPlan.js`.

/// A plan for loading a primitive.
pub struct PrimitiveLoadPlan {
    _private: (),
}

impl PrimitiveLoadPlan {
    /// Creates a new PrimitiveLoadPlan.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PrimitiveLoadPlan {
    fn default() -> Self { Self::new() }
}
