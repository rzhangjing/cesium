//! Ported from `packages/engine/Source/Scene/ResourceCacheKey.js`.

/// A key for the resource cache.
pub struct ResourceCacheKey {
    _private: (),
}

impl ResourceCacheKey {
    /// Creates a new ResourceCacheKey.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ResourceCacheKey {
    fn default() -> Self { Self::new() }
}
