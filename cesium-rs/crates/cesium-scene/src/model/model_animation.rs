//! Ported from `packages/engine/Source/Scene/ModelAnimation.js`.

/// A collection of model animations.
pub struct ModelAnimation {
    pub length: usize,
}

impl ModelAnimation {
    pub fn new() -> Self { Self { length: 0 } }
}

impl Default for ModelAnimation {
    fn default() -> Self { Self::new() }
}
