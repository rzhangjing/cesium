//! Ported from `packages/engine/Source/Scene/ResourceCacheKey.js`.

/// Resource cache key.
///
/// Unique identifier for a cached GPU resource.
pub struct ResourceCacheKey {
    /// The key string.
    pub key: String,
}

impl ResourceCacheKey {
    /// Creates a new ResourceCacheKey.
    pub fn new() -> Self { Self { key: String::new() } }
}

impl Default for ResourceCacheKey {
    fn default() -> Self { Self::new() }
}
