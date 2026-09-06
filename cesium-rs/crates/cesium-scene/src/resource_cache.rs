//! Ported from `packages/engine/Source/Scene/ResourceCache.js`.

/// Resource cache.
///
/// Caches GPU resources (textures, buffers) to avoid redundant uploads.
pub struct ResourceCache {
    /// The number of cached resources.
    pub cached_count: u32,
    /// Whether the cache is enabled.
    pub enabled: bool,
}

impl ResourceCache {
    /// Creates a new ResourceCache.
    pub fn new() -> Self { Self { cached_count: 0, enabled: true } }
}

impl Default for ResourceCache {
    fn default() -> Self { Self::new() }
}
