//! FIFO GPU cache with base-layer lock and live-entity deferral.
//!
//! Faithfully replicates `dynamic_globe.rs::evict_gpu_cache` (L1476-1502):
//!
//! ```text
//! while gpu_tex_order.len() > MAX_GPU_CACHE_ENTRIES {
//!     let old = gpu_tex_order.pop_front();
//!     if old.z <= BASE_LAYER_ZOOM { continue; }       // invariant 1
//!     if tile_entities.contains(old) {                // invariant 2
//!         gpu_tex_order.push_back(old); deferred++;
//!         continue;
//!     }
//!     remove_handles(old); evicted++;
//! }
//! ```
//!
//! Three invariants:
//! 1. BASE_LAYER permanent exemption (L1483)
//! 2. Live-entity push-back deferral (L1487-1491) — 花屏防护核心
//! 3. Termination: MAX_TILE_ENTITIES(1800) << MAX_GPU_CACHE_ENTRIES(3000)

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

use cesium_ports_driven::EvictionPolicy;

use crate::base_layer::BaseLayerGuard;

/// Result of an eviction pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct EvictionResult {
    /// Entries actually removed from the cache.
    pub evicted: u32,
    /// Entries deferred (pushed back) because they have live entities.
    pub deferred: u32,
}

/// FIFO-ordered GPU handle cache with eviction policy.
///
/// Generic over key type `K` (e.g. `(u32, u32, u32)` = TileKey).
/// Stores opaque handle values `V` (e.g. GPU texture IDs).
pub struct GpuCache<K, V>
where
    K: Hash + Eq + Copy,
{
    /// FIFO insertion order (oldest at front). Corresponds to `mgr.gpu_tex_order`.
    order: VecDeque<K>,
    /// Actual cached values. Corresponds to `mgr.gpu_textures` / `gpu_meshes` / etc.
    entries: HashMap<K, V>,
    /// Maximum entries before eviction kicks in (L73: 3000).
    max_entries: usize,
    /// Base layer guard for permanent exemption.
    base_guard: BaseLayerGuard,
    /// Zoom extractor: gets the zoom component from a key.
    zoom_of: fn(&K) -> u32,
    /// Keys currently backed by a live entity (deferral set for invariant 2).
    /// Corresponds to membership in `mgr.tile_entities` (L1487).
    live: HashSet<K>,
}

impl<K, V> GpuCache<K, V>
where
    K: Hash + Eq + Copy,
{
    /// Create a cache with the given capacity and zoom extractor.
    ///
    /// `zoom_of` extracts the zoom level from a key for base-layer checks.
    /// For `TileKey = (u32, u32, u32)`, this is `|k| k.2`.
    pub fn new(max_entries: usize, base_guard: BaseLayerGuard, zoom_of: fn(&K) -> u32) -> Self {
        Self {
            order: VecDeque::new(),
            entries: HashMap::new(),
            max_entries,
            base_guard,
            zoom_of,
            live: HashSet::new(),
        }
    }

    /// Mark a key as backed (or not) by a live entity.
    ///
    /// The host calls this when it spawns/despawns a tile entity so that
    /// `defer_if_live` (invariant 2, L1487) reflects the current live set.
    pub fn set_live(&mut self, key: K, is_live: bool) {
        if is_live {
            self.live.insert(key);
        } else {
            self.live.remove(&key);
        }
    }

    /// Insert a key-value pair. If the key already exists, updates the value
    /// without changing FIFO order (matches dynamic_globe behavior where
    /// re-uploads don't reset eviction priority).
    pub fn insert(&mut self, key: K, value: V) {
        if !self.entries.contains_key(&key) {
            self.order.push_back(key);
        }
        self.entries.insert(key, value);
    }

    /// Get a reference to a cached value.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    /// Check if a key is cached.
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Remove a specific key from the cache.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(v) = self.entries.remove(key) {
            // Remove from order queue (linear scan — acceptable for
            // explicit removals which are rare vs FIFO eviction).
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            Some(v)
        } else {
            None
        }
    }

    /// Current number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Run FIFO eviction until `len() <= max_entries`.
    ///
    /// `is_live` predicate corresponds to L1487: `mgr.tile_entities.contains_key(&old)`.
    /// Returns eviction/deferral counts for PerfCounters (M0.4 observation).
    ///
    /// Faithfully implements the three invariants:
    /// 1. Base-layer keys are skipped (never evicted).
    /// 2. Live-entity keys are pushed back (deferred).
    /// 3. Termination guaranteed because live entities << max_entries.
    pub fn evict<F>(&mut self, is_live: F) -> EvictionResult
    where
        F: Fn(&K) -> bool,
    {
        let mut result = EvictionResult::default();

        while self.order.len() > self.max_entries {
            let Some(old) = self.order.pop_front() else {
                break;
            };

            // Invariant 1: base layer permanent exemption (L1483)
            let zoom = (self.zoom_of)(&old);
            if self.base_guard.is_base_layer(zoom) {
                // Don't count as evicted — just skip. The entry stays in
                // `entries` but is removed from `order` (it will never be
                // evicted, so tracking order is pointless).
                continue;
            }

            // Invariant 2: live-entity deferral (L1487-1491)
            if is_live(&old) {
                self.order.push_back(old);
                result.deferred += 1;
                continue;
            }

            // Actually evict
            self.entries.remove(&old);
            result.evicted += 1;
        }

        result
    }

    /// Read-only access to the FIFO order (for `EvictionPolicy` trait impl).
    pub fn order(&self) -> &VecDeque<K> {
        &self.order
    }
}

/// `EvictionPolicy` contract impl (M1.1) expressing the three invariants.
///
/// The extra `Send + 'static` bounds (vs the inherent impl) are required by
/// the port trait; they are satisfied by `TileKey = (u32, u32, u32)`.
impl<K, V> EvictionPolicy<K> for GpuCache<K, V>
where
    K: Hash + Eq + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// Invariant: FIFO order, oldest first (`mgr.gpu_tex_order`, L1479).
    fn evict_order(&self) -> &VecDeque<K> {
        &self.order
    }

    /// Invariant 1 (L1483): base-layer tiles are never evicted.
    fn never_evict(&self, key: &K) -> bool {
        self.base_guard.is_base_layer((self.zoom_of)(key))
    }

    /// Invariant 2 (L1487): tiles with a live entity are deferred (pushed back).
    fn defer_if_live(&self, key: &K) -> bool {
        self.live.contains(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TileKey = (u32, u32, u32);

    fn zoom_of(k: &TileKey) -> u32 {
        k.2
    }

    fn make_cache(max: usize) -> GpuCache<TileKey, u64> {
        GpuCache::new(max, BaseLayerGuard::new(), zoom_of)
    }

    #[test]
    fn fifo_eviction_removes_oldest() {
        let mut cache = make_cache(3);
        cache.insert((0, 0, 4), 100);
        cache.insert((1, 0, 4), 200);
        cache.insert((2, 0, 4), 300);
        cache.insert((3, 0, 4), 400); // exceeds cap

        let result = cache.evict(|_| false);
        assert_eq!(result.evicted, 1);
        assert_eq!(result.deferred, 0);
        assert!(!cache.contains_key(&(0, 0, 4))); // oldest evicted
        assert!(cache.contains_key(&(3, 0, 4))); // newest kept
    }

    #[test]
    fn base_layer_never_evicted() {
        // With enough pressure, normal tiles get evicted but base layer survives.
        // Base-layer entries popped from order free slots (matching dynamic_globe
        // L1483: `continue` removes from gpu_tex_order but keeps GPU handles).
        let mut cache = make_cache(2);
        cache.insert((0, 0, 3), 1); // base layer (z=3)
        cache.insert((1, 1, 5), 2); // normal
        cache.insert((2, 2, 5), 3); // normal
        cache.insert((3, 3, 5), 4); // normal

        // order len=4 > max=2:
        // Pop (0,0,3): base → skip (freed from order, kept in entries)
        // order len=3 > 2: Pop (1,1,5): normal, not live → evict
        // order len=2, not > 2: exit
        let result = cache.evict(|_| false);
        assert_eq!(result.evicted, 1);
        assert!(cache.contains_key(&(0, 0, 3)));  // base layer NEVER evicted
        assert!(!cache.contains_key(&(1, 1, 5))); // evicted
        assert!(cache.contains_key(&(2, 2, 5)));  // still in cache
        assert!(cache.contains_key(&(3, 3, 5)));  // still in cache
    }

    #[test]
    fn live_entity_deferred() {
        let mut cache = make_cache(2);
        cache.insert((1, 1, 5), 10);
        cache.insert((2, 2, 5), 20);
        cache.insert((3, 3, 5), 30); // exceeds cap

        // Mark the oldest as "live"
        let result = cache.evict(|k| *k == (1, 1, 5));
        assert_eq!(result.deferred, 1);
        // (1,1,5) was pushed back, (2,2,5) evicted instead
        assert!(cache.contains_key(&(1, 1, 5)));
        assert!(!cache.contains_key(&(2, 2, 5)));
    }

    #[test]
    fn insert_duplicate_does_not_reorder() {
        let mut cache = make_cache(10);
        cache.insert((1, 1, 4), 100);
        cache.insert((2, 2, 4), 200);
        cache.insert((1, 1, 4), 999); // update value, keep position

        assert_eq!(cache.get(&(1, 1, 4)), Some(&999));
        assert_eq!(cache.order()[0], (1, 1, 4)); // still first
    }

    #[test]
    fn eviction_policy_trait_expresses_three_invariants() {
        // Exercise the ports `EvictionPolicy<K>` impl directly (dyn-compatible).
        let mut cache = make_cache(3000);
        cache.insert((0, 0, 3), 1); // base layer (z=3)
        cache.insert((1, 1, 5), 2); // normal
        cache.set_live((1, 1, 5), true);

        let policy: &dyn EvictionPolicy<TileKey> = &cache;
        // Invariant: FIFO order exposed.
        assert_eq!(policy.evict_order().len(), 2);
        // Invariant 1: base layer (z<=3) is never evicted.
        assert!(policy.never_evict(&(0, 0, 3)));
        assert!(!policy.never_evict(&(1, 1, 5)));
        // Invariant 2: live entity is deferred.
        assert!(policy.defer_if_live(&(1, 1, 5)));
        assert!(!policy.defer_if_live(&(0, 0, 3)));
    }
}
