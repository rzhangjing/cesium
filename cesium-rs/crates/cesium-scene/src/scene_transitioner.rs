//! Ported from `packages/engine/Source/Scene/SceneTransitioner.js`.

/// Scene transitioner.
///
/// Handles transitions between 3D, 2D, and Columbus View scene modes.
pub struct SceneTransitioner {
    /// Whether a transition is in progress.
    pub transitioning: bool,
    /// The transition duration in seconds.
    pub duration: f64,
}

impl SceneTransitioner {
    /// Creates a new SceneTransitioner.
    pub fn new() -> Self { Self { transitioning: false, duration: 3.0 } }
}

impl Default for SceneTransitioner {
    fn default() -> Self { Self::new() }
}
