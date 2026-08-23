//! Ported from `packages/engine/Source/Scene/SceneTransitioner.js`.

/// Transitions between scene modes.
pub struct SceneTransitioner {
    _private: (),
}

impl SceneTransitioner {
    /// Creates a new SceneTransitioner.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SceneTransitioner {
    fn default() -> Self { Self::new() }
}
