//! Ported from `packages/engine/Source/Scene/ModelAnimationChannel.js`.

/// A single channel within a model animation.
pub struct ModelAnimationChannel {
    pub target_path: String,
    pub sampler_index: u32,
}

impl ModelAnimationChannel {
    pub fn new() -> Self {
        Self { target_path: String::new(), sampler_index: 0 }
    }
}

impl Default for ModelAnimationChannel {
    fn default() -> Self { Self::new() }
}
