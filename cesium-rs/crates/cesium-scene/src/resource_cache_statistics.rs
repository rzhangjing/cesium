//! Ported from `packages/engine/Source/Scene/ResourceCacheStatistics.js`.

/// Statistics for the resource cache.
pub struct ResourceCacheStatistics {
    _private: (),
}

impl ResourceCacheStatistics {
    /// Creates a new ResourceCacheStatistics.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ResourceCacheStatistics {
    fn default() -> Self { Self::new() }
}
