//! Ported from `packages/engine/Source/Scene/PostProcessStageTextureCache.js`.

/// Post-process stage texture cache.
///
/// Caches intermediate textures for post-processing pipeline.
pub struct PostProcessStageTextureCache {
    /// Whether the cache is active.
    pub active: bool,
}

impl PostProcessStageTextureCache {
    /// Creates a new PostProcessStageTextureCache.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for PostProcessStageTextureCache {
    fn default() -> Self { Self::new() }
}
