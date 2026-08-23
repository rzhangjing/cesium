//! Ported from `packages/engine/Source/Scene/ModelAnimationLoop.js`.

/// Loop mode for model animations.
pub struct ModelAnimationLoop {
    _private: (),
}

impl ModelAnimationLoop {
    /// Creates a new ModelAnimationLoop.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelAnimationLoop {
    fn default() -> Self { Self::new() }
}
