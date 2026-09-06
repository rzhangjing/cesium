//! Ported from `packages/engine/Source/Scene/ImplicitSubtreeCache.js`.

/// Implicit subtree cache.
///
/// Caches loaded implicit subtrees for reuse.
pub struct ImplicitSubtreeCache {
    /// The maximum number of cached subtrees.
    pub max_cached: u32,
    /// The current cache size.
    pub cache_size: u32,
}

impl ImplicitSubtreeCache {
    /// Creates a new ImplicitSubtreeCache.
    pub fn new() -> Self { Self { max_cached: 100, cache_size: 0 } }
}

impl Default for ImplicitSubtreeCache {
    fn default() -> Self { Self::new() }
}
