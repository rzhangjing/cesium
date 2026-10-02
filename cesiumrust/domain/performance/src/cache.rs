//! 缓存系统：LRU 缓存、瓦片集缓存、引用计数资源缓存。
//!
//! 提供 LRU 缓存基元、瓦片集缓存 [`TilesetCache`]、引用计数资源缓存
//! [`ResourceCache`] 及其命中统计 [`CacheStatistics`]。

use std::collections::HashMap;
use std::hash::Hash;

/// 缓存统计跟踪：命中/未命中/驱逐次数的累计计数。
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    /// 缓存命中次数。
    pub hits: u64,
    /// 缓存未命中次数。
    pub misses: u64,
    /// 逐出次数。
    pub evictions: u64,
    /// 插入次数。
    pub insertions: u64,
    /// 当前条目数。
    pub entry_count: usize,
    /// 当前总大小（字节）。
    pub total_bytes: u64,
    /// 总大小峰值（字节）。
    pub peak_bytes: u64,
    /// 几何字节长度（顶点/索引缓冲）。
    pub geometry_byte_length: u64,
    /// 纹理字节长度。
    pub textures_byte_length: u64,
}

impl CacheStatistics {
    /// 创建新的统计。
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次缓存命中。
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// 记录一次缓存未命中。
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// 记录一次逐出。
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    /// 记录一次插入。
    pub fn record_insertion(&mut self) {
        self.insertions += 1;
    }

    /// 以 [0, 1]  fraction 返回命中率。
    pub fn hit_rate(&self) -> f64 {
        // 命中率 = 命中 / (命中 + 未命中)；无样本时回退 0.0，避免除零。
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }

    /// 重置所有统计。
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// 添加几何字节。
    pub fn add_geometry(&mut self, bytes: u64) {
        // 累计几何字节并刷新历史峰值，用于内存预算调参。
        self.geometry_byte_length += bytes;
        self.total_bytes += bytes;
        self.peak_bytes = self.peak_bytes.max(self.total_bytes);
    }

    /// 移除几何字节。
    pub fn remove_geometry(&mut self, bytes: u64) {
        self.geometry_byte_length = self.geometry_byte_length.saturating_sub(bytes);
        self.total_bytes = self.total_bytes.saturating_sub(bytes);
    }

    /// 添加纹理字节。
    pub fn add_texture(&mut self, bytes: u64) {
        self.textures_byte_length += bytes;
        self.total_bytes += bytes;
        self.peak_bytes = self.peak_bytes.max(self.total_bytes);
    }

    /// 移除纹理字节。
    pub fn remove_texture(&mut self, bytes: u64) {
        self.textures_byte_length = self.textures_byte_length.saturating_sub(bytes);
        self.total_bytes = self.total_bytes.saturating_sub(bytes);
    }
}

/// 一个通用的 LRU（最近最少使用）缓存。
///
/// 超出容量时逐出最近最少使用的条目。
#[derive(Debug, Clone)]
pub struct LruCache<K: Eq + Hash + Clone, V: Clone> {
    /// 最大条目数。
    capacity: usize,
    /// 存储。
    entries: HashMap<K, V>,
    /// 访问顺序（最近的在后）。
    order: Vec<K>,
    /// 统计。
    pub stats: CacheStatistics,
}

impl<K: Eq + Hash + Clone, V: Clone> LruCache<K, V> {
    /// 创建一个具有给定容量的新 LRU 缓存。
    pub fn new(capacity: usize) -> Self {
        // 容量下限钳到 1，确保逐出逻辑始终有可缓存槽位。
        Self {
            capacity: capacity.max(1),
            entries: HashMap::new(),
            order: Vec::new(),
            stats: CacheStatistics::new(),
        }
    }

    /// 获取容量。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 获取当前条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查缓存是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按键获取值（并将其标记为最近使用）。
    pub fn get(&mut self, key: &K) -> Option<&V> {
        // 命中计入统计并把键移到 order 尾部标记为最近使用。
        if self.entries.contains_key(key) {
            self.stats.record_hit();
            // 移到最近
            self.order.retain(|k| k != key);
            self.order.push(key.clone());
            self.entries.get(key)
        } else {
            self.stats.record_miss();
            None
        }
    }

    /// 获取值但不更新访问顺序。
    pub fn peek(&self, key: &K) -> Option<&V> {
        // 只读探查：不计入命中统计、也不改变 LRU 访问顺序。
        self.entries.get(key)
    }

    /// 插入一个键值对。
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        self.stats.record_insertion();

        if self.entries.contains_key(&key) {
            // 更新已有项
            self.order.retain(|k| k != &key);
            self.order.push(key.clone());
            return self.entries.insert(key, value);
        }

        // 已满则逐出
        if self.entries.len() >= self.capacity {
            self.evict_lru();
        }

        self.order.push(key.clone());
        self.entries.insert(key, value);
        self.stats.entry_count = self.entries.len();
        None
    }

    /// 从缓存中移除一个键。
    pub fn remove(&mut self, key: &K) -> Option<V> {
        // 同步从访问顺序中剔除该键，避免遗留陈旧键。
        self.order.retain(|k| k != key);
        let value = self.entries.remove(key);
        self.stats.entry_count = self.entries.len();
        value
    }

    /// 检查某个键是否存在。
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// 逐出最近最少使用的条目。
    fn evict_lru(&mut self) -> Option<(K, V)> {
        // order 尾部为最近使用，首部即最近最少使用，逐出取首部键。
        if let Some(lru_key) = self.order.first().cloned() {
            self.order.remove(0);
            let value = self.entries.remove(&lru_key);
            self.stats.record_eviction();
            self.stats.entry_count = self.entries.len();
            return value.map(|v| (lru_key, v));
        }
        None
    }

    /// 清除所有条目。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.stats.entry_count = 0;
    }

    /// 以 LRU 顺序获取所有键（最久未用在前）。
    pub fn keys_lru_order(&self) -> &[K] {
        &self.order
    }
}

/// 一个带大小跟踪的瓦片缓存条目。
#[derive(Debug, Clone)]
pub struct TileCacheEntry {
    /// 瓦片标识。
    pub tile_id: u64,
    /// 内存大小（字节）。
    pub size_bytes: u64,
    /// 本瓦片本帧是否被触用（使用）。
    pub touched: bool,
    /// 上次触用时的帧编号。
    pub last_touched_frame: u64,
}

/// 基于哨兵（sentinel）的 LRU 逐出的瓦片集缓存。
///
/// 瓦片分为两组：
/// - 未触用（逐出候选，按 LRU 顺序）
/// - 本帧已触用（受保护不被逐出）
#[derive(Debug, Clone)]
pub struct TilesetCache {
    /// 所有已缓存瓦片。
    tiles: Vec<TileCacheEntry>,
    /// 最大缓存大小（字节）。
    pub cache_bytes: u64,
    /// 当前总内存使用量。
    pub total_memory_bytes: u64,
    /// 下次卸载时是否修剪全部瓦片。
    trim_tiles: bool,
    /// 统计。
    pub stats: CacheStatistics,
}

impl Default for TilesetCache {
    /// 返回缺省瓦片集缓存：空瓦片表、上限 512 MB。
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
    /// 创建一个具有字节预算的新瓦片集缓存。
    pub fn new(cache_bytes: u64) -> Self {
        Self {
            cache_bytes,
            ..Default::default()
        }
    }

    /// 为新帧重置缓存。
    /// 所有瓦片都成为逐出候选。
    pub fn reset(&mut self) {
        // 帧起始清空触用标记：本帧未再 touch 的瓦片随即重获逐出资格。
        for tile in &mut self.tiles {
            tile.touched = false;
        }
    }

    /// 触用一个瓦片（标记为本帧已用）。
    pub fn touch(&mut self, tile_id: u64, frame_number: u64) {
        // 命中本帧瓦片则刷新其触用帧；未命中仅计入统计，不新建条目。
        if let Some(tile) = self.tiles.iter_mut().find(|t| t.tile_id == tile_id) {
            tile.touched = true;
            tile.last_touched_frame = frame_number;
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }
    }

    /// 向缓存添加一个瓦片。
    pub fn add(&mut self, tile_id: u64, size_bytes: u64, frame_number: u64) {
        // 重复瓦片直接跳过，避免同一 tile_id 多次计入内存总量。
        if self.tiles.iter().any(|t| t.tile_id == tile_id) {
            return; // 已缓存
        }

        // 新瓦片默认本帧已触用，受保护不被本轮逐出。
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

    /// 从缓存中移除特定瓦片。
    pub fn remove(&mut self, tile_id: u64) -> Option<TileCacheEntry> {
        // 移除并回收字节，saturating 防止内存总量下溢。
        if let Some(idx) = self.tiles.iter().position(|t| t.tile_id == tile_id) {
            let tile = self.tiles.remove(idx);
            self.total_memory_bytes = self.total_memory_bytes.saturating_sub(tile.size_bytes);
            self.stats.entry_count = self.tiles.len();
            Some(tile)
        } else {
            None
        }
    }

    /// 卸载超出缓存预算的瓦片。
    /// 返回被逐出瓦片的 ID。
    pub fn unload_tiles(&mut self) -> Vec<u64> {
        let mut evicted = Vec::new();
        let trim_all = self.trim_tiles;
        self.trim_tiles = false;

        // 将未触用瓦片按 last_touched_frame 排序（LRU 在前）
        let mut untouched: Vec<usize> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.touched)
            .map(|(i, _)| i)
            .collect();
        untouched.sort_by_key(|&i| self.tiles[i].last_touched_frame);

        // 从 LRU 开始逐出，直到降到预算内（或修剪全部）
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

    /// 强制下次卸载时修剪全部瓦片。
    pub fn trim(&mut self) {
        self.trim_tiles = true;
    }

    /// 获取已缓存瓦片数。
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// 检查某个瓦片是否已缓存。
    pub fn contains(&self, tile_id: u64) -> bool {
        self.tiles.iter().any(|t| t.tile_id == tile_id)
    }

    /// 获取本帧被触用的瓦片数。
    pub fn touched_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.touched).count()
    }
}

/// 一个引用计数的缓存条目。
#[derive(Debug, Clone)]
pub struct RefCountedEntry<V: Clone> {
    /// 缓存的值。
    pub value: V,
    /// 引用计数。
    pub reference_count: u32,
    /// 大小（字节，用于内存跟踪）。
    pub size_bytes: u64,
}

/// 引用计数的资源缓存。
///
/// 资源被共享并引用计数；当引用计数降为零时
/// 将其移除。
#[derive(Debug, Clone)]
pub struct ResourceCache<K: Eq + Hash + Clone, V: Clone> {
    /// 缓存条目。
    entries: HashMap<K, RefCountedEntry<V>>,
    /// 统计。
    pub stats: CacheStatistics,
}

impl<K: Eq + Hash + Clone, V: Clone> Default for ResourceCache<K, V> {
    /// 返回空资源缓存：无条目、零统计。
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            stats: CacheStatistics::new(),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> ResourceCache<K, V> {
    /// 创建一个新的资源缓存。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从缓存获取资源（引用计数加一）。
    pub fn get(&mut self, key: &K) -> Option<&V> {
        // 取用即增引用，防止共享资源在使用期间被 release 归零移除。
        if let Some(entry) = self.entries.get_mut(key) {
            entry.reference_count += 1;
            self.stats.record_hit();
            Some(&entry.value)
        } else {
            self.stats.record_miss();
            None
        }
    }

    /// 向缓存添加资源。
    pub fn add(&mut self, key: K, value: V, size_bytes: u64) -> bool {
        if self.entries.contains_key(&key) {
            return false; // 已存在
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

    /// 释放一个资源的引用。
    /// 如果资源被移除（引用计数降为 0）则返回 true。
    pub fn release(&mut self, key: &K) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.reference_count = entry.reference_count.saturating_sub(1);
            // 引用递减到 0 时真正淘汰条目并回收其字节占用。
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

    /// 获取某个键的引用计数。
    pub fn reference_count(&self, key: &K) -> u32 {
        self.entries.get(key).map(|e| e.reference_count).unwrap_or(0)
    }

    /// 检查某个键是否存在于缓存中。
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// 获取条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查缓存是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 清除所有条目。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.stats.entry_count = 0;
        self.stats.total_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === CacheStatistics 测试 ===

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
        assert_eq!(stats.peak_bytes, 1500); // 峰值不变
    }

    // === LruCache 测试 ===

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
        cache.put("c", 3); // 应逐出 "a"

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

        // 访问 "a" 使其成为最近使用
        cache.get(&"a");

        // 插入 "c" —— 应逐出 "b"（最近最少使用）
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

        // Peek 不更新顺序
        assert_eq!(cache.peek(&"a"), Some(&1));

        // 插入 "c" —— 仍应逐出 "a"，因为 peek 未更新顺序
        cache.put("c", 3);
        assert_eq!(cache.peek(&"a"), None);
    }

    #[test]
    fn test_lru_cache_stats() {
        let mut cache = LruCache::new(2);
        cache.put("a", 1);
        cache.get(&"a"); // 命中
        cache.get(&"x"); // 未命中

        assert_eq!(cache.stats.hits, 1);
        assert_eq!(cache.stats.misses, 1);
        assert_eq!(cache.stats.insertions, 1);
    }

    // === TilesetCache 测试 ===

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
        cache.add(3, 200, 0); // 总计 600 > 500

        // 重置并仅触用瓦片 3
        cache.reset();
        cache.touch(3, 1);

        let evicted = cache.unload_tiles();
        // 应逐出瓦片 1（未触用，LRU）以降到预算内
        assert!(evicted.contains(&1));
        // 瓦片 3 已触用，永不逐出
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
        cache.trim(); // 强制修剪全部

        let evicted = cache.unload_tiles();
        assert_eq!(evicted.len(), 2);
        assert_eq!(cache.tile_count(), 0);
    }

    // === ResourceCache 测试 ===

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
        // 重复添加应失败
        assert!(!cache.add("key".to_string(), 99, 4));
        // 值不变
        assert_eq!(cache.get(&"key".to_string()), Some(&42));
    }

    #[test]
    fn test_resource_cache_ref_counting() {
        let mut cache: ResourceCache<String, i32> = ResourceCache::new();
        cache.add("key".to_string(), 42, 4);

        // 初始引用计数为 1
        assert_eq!(cache.reference_count(&"key".to_string()), 1);

        // get 使计数递增
        cache.get(&"key".to_string());
        assert_eq!(cache.reference_count(&"key".to_string()), 2);

        // release 使计数递减
        assert!(!cache.release(&"key".to_string()));
        assert_eq!(cache.reference_count(&"key".to_string()), 1);

        // 释放至零则移除
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
        assert_eq!(cache.stats.peak_bytes, 300); // 峰值不变
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
