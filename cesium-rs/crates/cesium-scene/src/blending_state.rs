//! Ported from `packages/engine/Source/Scene/BlendingState.js`.

/// Blending state.
pub struct BlendingState {
    _private: (),
}

impl BlendingState {
    /// Creates a new BlendingState.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BlendingState {
    fn default() -> Self { Self::new() }
}
