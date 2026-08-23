//! Ported from `packages/engine/Source/Scene/PrimitiveState.js`.

/// The loading state of a primitive.
pub struct PrimitiveState {
    _private: (),
}

impl PrimitiveState {
    /// Creates a new PrimitiveState.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PrimitiveState {
    fn default() -> Self { Self::new() }
}
