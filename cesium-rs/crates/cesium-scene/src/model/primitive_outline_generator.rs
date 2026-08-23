//! Ported from `packages/engine/Source/Scene/Model/PrimitiveOutlineGenerator.js`.

/// Generates outlines for primitives.
pub struct PrimitiveOutlineGenerator {
    _private: (),
}

impl PrimitiveOutlineGenerator {
    /// Creates a new PrimitiveOutlineGenerator.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PrimitiveOutlineGenerator {
    fn default() -> Self { Self::new() }
}
