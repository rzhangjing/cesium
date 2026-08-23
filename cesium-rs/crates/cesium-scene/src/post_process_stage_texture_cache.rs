//! Ported from `packages/engine/Source/Scene/PostProcessStageTextureCache.js`.

/// Texture cache for post-process stages.
pub struct PostProcessStageTextureCache {
    _private: (),
}

impl PostProcessStageTextureCache {
    /// Creates a new PostProcessStageTextureCache.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PostProcessStageTextureCache {
    fn default() -> Self { Self::new() }
}
