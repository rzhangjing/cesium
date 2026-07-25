//! Cache systems: LRU cache, tileset cache, reference-counted resource cache.
//!
//! Maps to CesiumJS:
//! - `Scene/Cesium3DTilesetCache.js` → TilesetCache
//! - `Scene/ResourceCache.js` → ResourceCache
//! - `Scene/ResourceCacheStatistics.js` → CacheStatistics

use std::collections::HashMap;
use std::hash::Hash;

/// Cache statistics tracking.
///
/// Maps to CesiumJS `ResourceCacheStatistics.js`.
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of evictions.
    pub evictions: u64,
    /// Number of insertions.
    pub insertions: u64,
    /// Current number of entries.
    pub entry_count: usize,
    /// Current total size in bytes.
    pub total_bytes: u64,
    /// Peak total size in bytes.
    pub peak_bytes: u64,
    /// Geometry byte length (vertex/index buffers).
    pub geometry_byte_length: u64,
    /// Texture byte length.
    pub textures_byte_length: u64,
}

impl CacheStatistics {
    /// Create new statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// Record a cache miss.
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// Record an eviction.
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    /// Record an insertion.
    pub fn record_insertion(&mut self) {
        self.insertions += 1;
    }

    /// Get the hit rate as a fraction [0, 1].
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }

    /// Reset all statistics.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Add geometry bytes.
    pub fn add_geometry(&mut self, bytes: u64) {
        self.geometry_byte_length += bytes;
        self.total_bytes += bytes;
        self.peak_bytes = self.peak_bytes.max(self.total_bytes);
    }

    /// Remove geometry bytes.
    pub fn remove_geometry(&mut self, bytes: u64) {
        self.geometry_byte_length = self.geometry_byte_length.saturating_sub(bytes);
        self.total_bytes = self.total_bytes.saturating_sub(bytes);
    }

    /// Add texture bytes.
    pub fn add_texture(&mut self, bytes: u64) {
        self.textures_byte_length += bytes;
        self.total_bytes += bytes;
        self.peak_bytes = self.peak_bytes.max(self.total_bytes);
    }

    /// Remove texture bytes.
    pub fn remove_texture(&mut self, bytes: u64) {
        self.textures_byte_length = self.textures_byte_length.saturating_sub(bytes);
        self.total_bytes = self.total_bytes.saturating_sub(bytes);
    }
}

/// A generic LRU (Least Recently Used) cache.
///
/// Evicts the least recently used entry when capacity is exceeded.
#[derive(Debug, Clone)]
pub struct LruCache<K: Eq + Hash + Clone, V: Clone> {
    /// Maximum number of entries.
    capacity: usize,
    /// Storage.
    entries: HashMap<K, V>,
    /// Access order (most recent at back).
    order: Vec<K>,
    /// Statistics.
    pub stats: CacheStatistics,
}

impl<K: Eq + Hash + Clone, V: Clone> LruCache<K, V> {
    /// Create a new LRU cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: HashMap::new(),
            order: Vec::new(),
            stats: CacheStatistics::new(),
        }
    }

    /// Get the capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the current number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get a value by key (marks as recently used).
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.entries.contains_key(key) {
            self.stats.record_hit();
            // Move to most recent
            self.order.retain(|k| k != key);
            self.order.push(key.clone());
            self.entries.get(key)
        } else {
            self.stats.record_miss();
            None
        }
    }

    /// Get a value without updating access order.
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    /// Insert a key-value pair.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        self.stats.record_insertion();

        if self.entries.contains_key(&key) {
            // Update existing
            self.order.retain(|k| k != &key);
            self.order.push(key.clone());
            return self.entries.insert(key, value);
        }

        // Evict if at capacity
        if self.entries.len() >= self.capacity {
            self.evict_lru();
        }

        self.order.push(key.clone());
        self.entries.insert(key, value);
        self.stats.entry_count = self.entries.len();
        None
    }

    /// Remove a key from the cache.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.order.retain(|k| k != key);
        let value = self.entries.remove(key);
        self.stats.entry_count = self.entries.len();
        value
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Evict the least recently used entry.
    fn evict_lru(&mut self) -> Option<(K, V)> {
        if let Some(lru_key) = self.order.first().cloned() {
            self.order.remove(0);
            let value = self.entries.remove(&lru_key);
            self.stats.record_eviction();
            self.stats.entry_count = self.entries.len();
            return value.map(|v| (lru_key, v));
        }
        None
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.stats.entry_count = 0;
    }

    /// Get all keys in LRU order (least recent first).
    pub fn keys_lru_order(&self) -> &[K] {
        &self.order
    }
}

/// A tile cache entry with size tracking.
#[derive(Debug, Clone)]
pub struct TileCacheEntry {
    /// Tile identifier.
    pub tile_id: u64,
    /// Memory size in bytes.
    pub size_bytes: u64,
    /// Whether this tile was touched (used) this frame.
    pub touched: bool,
    /// Frame number when last touched.
    pub last_touched_frame: u64,
}

/// Tileset cache with sentinel-based LRU eviction.
///
/// Maps to CesiumJS `Cesium3DTilesetCache.js`.
/// Tiles are divided into two groups:
/// - Untouched (candidates for eviction, LRU order)
/// - Touched this frame (protected from eviction)
#[derive(Debug, Clone)]
pub struct TilesetCache {
    /// All cached tiles.
    tiles: Vec<TileCacheEntry>,
    /// Maximum cache size in bytes.
    pub cache_bytes: u64,
    /// Current total memory usage.
    pub total_memory_bytes: u64,
    /// Whether to trim all tiles on next unload.
    trim_tiles: bool,
    /// Statistics.
    pub stats: CacheStatistics,
}

impl Default for TilesetCache {
    fn default() -> Self {
        Self {
            tiles: Vec::new(),
            cache_bytes: 512 * 1024 * 1024, // 512 MB
            total_memory_bytes: 0,
            trim_tiles: false,
            stats: CacheStatistics::new(),
        }
    }
}

impl TilesetCache {
    /// Create a new tileset cache with a byte budget.
    pub fn new(cache_bytes: u64) -> Self {
        Self {
            cache_bytes,
            ..Default::default()
        }
    }

    /// Reset the cache for a new frame.
    /// All tiles become candidates for eviction.
    pub fn reset(&mut self) {
        for tile in &mut self.tiles {
            tile.touched = false;
        }
    }

    /// Touch a tile (mark as used this frame).
    pub fn touch(&mut self, tile_id: u64, frame_number: u64) {
        if let Some(tile) = self.tiles.iter_mut().find(|t| t.tile_id == tile_id) {
            tile.touched = true;
            tile.last_touched_frame = frame_number;
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }
    }

    /// Add a tile to the cache.
    pub fn add(&mut self, tile_id: u64, size_bytes: u64, frame_number: u64) {
        if self.tiles.iter().any(|t| t.tile_id == tile_id) {
            return; // Already cached
        }

        self.tiles.push(TileCacheEntry {
            tile_id,
            size_bytes,
            touched: true,
            last_touched_frame: frame_number,
        });
        self.total_memory_bytes += size_bytes;
        self.stats.record_insertion();
        self.stats.entry_count = self.tiles.len();
    }

    /// Remove a specific tile from the cache.
    pub fn remove(&mut self, tile_id: u64) -> Option<TileCacheEntry> {
        if let Some(idx) = self.tiles.iter().position(|t| t.tile_id == tile_id) {
            let tile = self.tiles.remove(idx);
            self.total_memory_bytes = self.total_memory_bytes.saturating_sub(tile.size_bytes);
            self.stats.entry_count = self.tiles.len();
            Some(tile)
        } else {
            None
        }
    }

    /// Unload tiles that exceed the cache budget.
    /// Returns the IDs of evicted tiles.
    pub fn unload_tiles(&mut self) -> Vec<u64> {
        let mut evicted = Vec::new();
        let trim_all = self.trim_tiles;
        self.trim_tiles = false;

        // Sort untouched tiles by last_touched_frame (LRU first)
        let mut untouched: Vec<usize> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.touched)
            .map(|(i, _)| i)
            .collect();
        untouched.sort_by_key(|&i| self.tiles[i].last_touched_frame);

        // Evict from LRU until under budget (or trim all)
        let mut to_remove = Vec::new();
        for &idx in &untouched {
            if !trim_all && self.total_memory_bytes <= self.cache_bytes {
                break;
            }
            let tile_id = self.tiles[idx].tile_id;
            let size = self.tiles[idx].size_bytes;
            self.total_memory_bytes = self.total_memory_bytes.saturating_sub(size);
            self.stats.record_eviction();
            evicted.push(tile_id);
            to_remove.push(tile_id);
        }

        self.tiles.retain(|t| !to_remove.contains(&t.tile_id));
        self.stats.entry_count = self.tiles.len();
        evicted
    }

    /// Force trim all tiles on next unload.
    pub fn trim(&mut self) {
        self.trim_tiles = true;
    }

    /// Get the number of cached tiles.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Check if a tile is cached.
    pub fn contains(&self, tile_id: u64) -> bool {
        self.tiles.iter().any(|t| t.tile_id == tile_id)
    }

    /// Get the number of touched tiles this frame.
    pub fn touched_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.touched).count()
    }
}

/// A reference-counted cache entry.
#[derive(Debug, Clone)]
pub struct RefCountedEntry<V: Clone> {
    /// The cached value.
    pub value: V,
    /// Reference count.
    pub reference_count: u32,
    /// Size in bytes (for memory tracking).
    pub size_bytes: u64,
}

/// Reference-counted resource cache.
///
/// Maps to CesiumJS `ResourceCache.js`.
/// Resources are shared and reference-counted; they are removed
/// when the reference count drops to zero.
#[derive(Debug, Clone)]
pub struct ResourceCache<K: Eq + Hash + Clone, V: Clone> {
    /// Cache entries.
    entries: HashMap<K, RefCountedEntry<V>>,
    /// Statistics.
    pub stats: CacheStatistics,
}

impl<K: Eq + Hash + Clone, V: Clone> Default for ResourceCache<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            stats: CacheStatistics::new(),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> ResourceCache<K, V> {
    /// Create a new resource cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a resource from the cache (increments reference count).
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.reference_count += 1;
            self.stats.record_hit();
            Some(&entry.value)
        } else {
            self.stats.record_miss();
            None
        }
    }

    /// Add a resource to the cache.
    pub fn add(&mut self, key: K, value: V, size_bytes: u64) -> bool {
        if self.entries.contains_key(&key) {
            return false; // Already exists
        }

        self.entries.insert(
            key,
            RefCountedEntry {
                value,
                reference_count: 1,
                size_bytes,
            },
        );
        self.stats.record_insertion();
        self.stats.entry_count = self.entries.len();
        self.stats.total_bytes += size_bytes;
        self.stats.peak_bytes = self.stats.peak_bytes.max(self.stats.total_bytes);
        true
    }

    /// Release a reference to a resource.
    /// Returns true if the resource was removed (ref count reached 0).
    pub fn release(&mut self, key: &K) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.reference_count = entry.reference_count.saturating_sub(1);
            if entry.reference_count == 0 {
                let size = entry.size_bytes;
                self.entries.remove(key);
                self.stats.entry_count = self.entries.len();
                self.stats.total_bytes = self.stats.total_bytes.saturating_sub(size);
                return true;
            }
        }
        false
    }

    /// Get the reference count for a key.
    pub fn reference_count(&self, key: &K) -> u32 {
        self.entries.get(key).map(|e| e.reference_count).unwrap_or(0)
    }

    /// Check if a key exists in the cache.
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.stats.entry_count = 0;
        self.stats.total_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === CacheStatistics tests ===

    #[test]
    fn test_statistics_default() {
        let stats = CacheStatistics::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_statistics_hit_rate() {
        let mut stats = CacheStatistics::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        assert!((stats.hit_rate() - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_statistics_memory() {
        let mut stats = CacheStatistics::new();
        stats.add_geometry(1000);
        stats.add_texture(500);
        assert_eq!(stats.total_bytes, 1500);
        assert_eq!(stats.peak_bytes, 1500);

        stats.remove_geometry(400);
        assert_eq!(stats.total_bytes, 1100);
        assert_eq!(stats.peak_bytes, 1500); // Peak unchanged
    }

    // === LruCache tests ===

    #[test]
    fn test_lru_cache_basic() {
        let mut cache = LruCache::new(3);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"d"), None);
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3); // Should evict "a"

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), Some(&3));
    }

    #[test]
    fn test_lru_cache_access_order() {
        let mut cache = LruCache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);

        // Access "a" to make it recently used
        cache.get(&"a");

        // Insert "c" - should evict "b" (least recently used)
        cache.put("c", 3);

        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(&3));
    }

    #[test]
    fn test_lru_cache_update() {
        let mut cache = LruCache::new(2);
        cache.put("a", 1);
        let old = cache.put("a", 10);
        assert_eq!(old, Some(1));
        assert_eq!(cache.get(&"a"), Some(&10));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_cache_remove() {
        let mut cache = LruCache::new(3);
        cache.put("a", 1);
        cache.put("b", 2);

        let removed = cache.remove(&"a");
        assert_eq!(removed, Some(1));
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains(&"a"));
    }

    #[test]
    fn test_lru_cache_clear() {
        let mut cache = LruCache::new(3);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_lru_cache_peek() {
        let mut cache = LruCache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);

        // Peek doesn't update order
        assert_eq!(cache.peek(&"a"), Some(&1));

        // Insert "c" - should still evict "a" since peek didn't update
        cache.put("c", 3);
        assert_eq!(cache.peek(&"a"), None);
    }

    #[test]
    fn test_lru_cache_stats() {
        let mut cache = LruCache::new(2);
        cache.put("a", 1);
        cache.get(&"a"); // hit
        cache.get(&"x"); // miss

        assert_eq!(cache.stats.hits, 1);
        assert_eq!(cache.stats.misses, 1);
        assert_eq!(cache.stats.insertions, 1);
    }

    // === TilesetCache tests ===

    #[test]
    fn test_tileset_cache_basic() {
        let mut cache = TilesetCache::new(1000);
        cache.add(1, 100, 0);
        cache.add(2, 200, 0);

        assert_eq!(cache.tile_count(), 2);
        assert_eq!(cache.total_memory_bytes, 300);
        assert!(cache.contains(1));
        assert!(!cache.contains(3));
    }

    #[test]
    fn test_tileset_cache_touch() {
        let mut cache = TilesetCache::new(1000);
        cache.add(1, 100, 0);
        cache.reset();
        assert_eq!(cache.touched_count(), 0);

        cache.touch(1, 1);
        assert_eq!(cache.touched_count(), 1);
    }

    #[test]
    fn test_tileset_cache_eviction() {
        let mut cache = TilesetCache::new(500);
        cache.add(1, 200, 0);
        cache.add(2, 200, 0);
        cache.add(3, 200, 0); // Total 600 > 500

        // Reset and touch only tile 3
        cache.reset();
        cache.touch(3, 1);

        let evicted = cache.unload_tiles();
        // Should evict tile 1 (untouched, LRU) to get under budget
        assert!(evicted.contains(&1));
        // Tile 3 is touched, never evicted
        assert!(!evicted.contains(&3));
        assert!(cache.total_memory_bytes <= 500);
        assert!(cache.contains(3));
    }

    #[test]
    fn test_tileset_cache_remove() {
        let mut cache = TilesetCache::new(1000);
        cache.add(1, 100, 0);
        cache.add(2, 200, 0);

        let removed = cache.remove(1);
        assert!(removed.is_some());
        assert_eq!(cache.tile_count(), 1);
        assert_eq!(cache.total_memory_bytes, 200);
    }

    #[test]
    fn test_tileset_cache_trim() {
        let mut cache = TilesetCache::new(10000);
        cache.add(1, 100, 0);
        cache.add(2, 100, 0);
        cache.reset();
        cache.trim(); // Force trim all

        let evicted = cache.unload_tiles();
        assert_eq!(evicted.len(), 2);
        assert_eq!(cache.tile_count(), 0);
    }

    // === ResourceCache tests ===

    #[test]
    fn test_resource_cache_basic() {
        let mut cache: ResourceCache<String, Vec<u8>> = ResourceCache::new();
        assert!(cache.add("buf1".to_string(), vec![1, 2, 3], 3));
        assert!(cache.contains(&"buf1".to_string()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_resource_cache_duplicate() {
        let mut cache: ResourceCache<String, i32> = ResourceCache::new();
        assert!(cache.add("key".to_string(), 42, 4));
        // Duplicate should fail
        assert!(!cache.add("key".to_string(), 99, 4));
        // Value unchanged
        assert_eq!(cache.get(&"key".to_string()), Some(&42));
    }

    #[test]
    fn test_resource_cache_ref_counting() {
        let mut cache: ResourceCache<String, i32> = ResourceCache::new();
        cache.add("key".to_string(), 42, 4);

        // Initial ref count is 1
        assert_eq!(cache.reference_count(&"key".to_string()), 1);

        // Get increments
        cache.get(&"key".to_string());
        assert_eq!(cache.reference_count(&"key".to_string()), 2);

        // Release decrements
        assert!(!cache.release(&"key".to_string()));
        assert_eq!(cache.reference_count(&"key".to_string()), 1);

        // Release to zero removes
        assert!(cache.release(&"key".to_string()));
        assert!(!cache.contains(&"key".to_string()));
    }

    #[test]
    fn test_resource_cache_stats() {
        let mut cache: ResourceCache<String, i32> = ResourceCache::new();
        cache.add("a".to_string(), 1, 100);
        cache.add("b".to_string(), 2, 200);

        assert_eq!(cache.stats.total_bytes, 300);
        assert_eq!(cache.stats.peak_bytes, 300);
        assert_eq!(cache.stats.entry_count, 2);

        cache.release(&"a".to_string());
        assert_eq!(cache.stats.total_bytes, 200);
        assert_eq!(cache.stats.peak_bytes, 300); // Peak unchanged
    }

    #[test]
    fn test_resource_cache_clear() {
        let mut cache: ResourceCache<String, i32> = ResourceCache::new();
        cache.add("a".to_string(), 1, 100);
        cache.add("b".to_string(), 2, 200);
        cache.clear();

        assert!(cache.is_empty());
        assert_eq!(cache.stats.total_bytes, 0);
    }
}
