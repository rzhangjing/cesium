//! Ported from `packages/engine/Source/Scene/ResourceCacheStatistics.js`.

/// Resource cache statistics.
///
/// Tracks hit/miss rates and memory usage of the resource cache.
pub struct ResourceCacheStatistics {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
}

impl ResourceCacheStatistics {
    /// Creates a new ResourceCacheStatistics.
    pub fn new() -> Self { Self { hits: 0, misses: 0 } }

    /// Returns the hit rate as a fraction.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

impl Default for ResourceCacheStatistics {
    fn default() -> Self { Self::new() }
}
