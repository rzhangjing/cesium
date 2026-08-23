//! Ported from `packages/engine/Source/Scene/TweenCollection.js`.

/// A collection of tween animations.
pub struct TweenCollection {
    _private: (),
}

impl TweenCollection {
    /// Creates a new TweenCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TweenCollection {
    fn default() -> Self { Self::new() }
}
