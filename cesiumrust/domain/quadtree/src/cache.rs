//! Tile cache and loading queue management.
//!
//! Maps to CesiumJS tile loading/caching:
//! - LRU cache for loaded tiles
//! - Priority-based loading queue
//! - Tile replacement policies

use std::collections::{HashMap, VecDeque};

/// A tile identifier (x, y, level).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileId {
    /// Tile X coordinate.
    pub x: u32,
    /// Tile Y coordinate.
    pub y: u32,
    /// Tile level.
    pub level: u32,
}

impl TileId {
    /// Creates a new tile ID.
    pub fn new(x: u32, y: u32, level: u32) -> Self {
        Self { x, y, level }
    }
}

/// Tile loading priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TilePriority {
    /// Low priority (preload).
    Low = 0,
    /// Normal priority.
    #[default]
    Normal = 1,
    /// High priority (visible).
    High = 2,
    /// Critical priority (center of view).
    Critical = 3,
}

/// A tile in the loading queue.
#[derive(Debug, Clone)]
pub struct QueuedTile {
    /// Tile identifier.
    pub id: TileId,
    /// Loading priority.
    pub priority: TilePriority,
    /// Frame number when queued.
    pub frame_number: u64,
    /// Distance from camera (for priority sorting).
    pub distance: f64,
}

/// Loading queue for tiles.
///
/// Manages the order in which tiles are loaded based on priority and distance.
#[derive(Debug, Default)]
pub struct TileLoadQueue {
    /// Queued tiles.
    queue: VecDeque<QueuedTile>,
    /// Maximum queue size.
    max_size: usize,
}

impl TileLoadQueue {
    /// Creates a new loading queue.
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
        }
    }

    /// Adds a tile to the queue.
    pub fn enqueue(&mut self, tile: QueuedTile) {
        if self.queue.len() >= self.max_size {
            // Remove lowest priority tile
            if let Some(min_idx) = self
                .queue
                .iter()
                .enumerate()
                .min_by_key(|(_, t)| (t.priority, -(t.distance as i64)))
                .map(|(i, _)| i)
            {
                self.queue.remove(min_idx);
            }
        }
        self.queue.push_back(tile);
    }

    /// Gets the next tile to load (highest priority, closest).
    pub fn dequeue(&mut self) -> Option<QueuedTile> {
        if self.queue.is_empty() {
            return None;
        }

        // Find highest priority, closest tile
        let best_idx = self
            .queue
            .iter()
            .enumerate()
            .max_by_key(|(_, t)| (t.priority, -(t.distance as i64)))
            .map(|(i, _)| i)?;

        self.queue.remove(best_idx)
    }

    /// Returns the number of queued tiles.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Clears the queue.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

/// LRU cache for loaded tiles.
///
/// Maps to CesiumJS tile cache behavior.
#[derive(Debug)]
pub struct TileCache<T> {
    /// Cached tiles.
    tiles: HashMap<TileId, T>,
    /// Access order (most recent at back).
    access_order: VecDeque<TileId>,
    /// Maximum cache size.
    max_size: usize,
    /// Evicted tiles (for cleanup).
    evicted: Vec<(TileId, T)>,
}

impl<T> TileCache<T> {
    /// Creates a new tile cache.
    pub fn new(max_size: usize) -> Self {
        Self {
            tiles: HashMap::new(),
            access_order: VecDeque::new(),
            max_size,
            evicted: Vec::new(),
        }
    }

    /// Gets a tile from the cache.
    pub fn get(&mut self, id: &TileId) -> Option<&T> {
        if self.tiles.contains_key(id) {
            // Update access order
            self.access_order.retain(|x| x != id);
            self.access_order.push_back(*id);
            self.tiles.get(id)
        } else {
            None
        }
    }

    /// Inserts a tile into the cache.
    pub fn insert(&mut self, id: TileId, tile: T) {
        // Evict if necessary
        while self.tiles.len() >= self.max_size {
            if let Some(oldest) = self.access_order.pop_front() {
                if let Some(evicted_tile) = self.tiles.remove(&oldest) {
                    self.evicted.push((oldest, evicted_tile));
                }
            } else {
                break;
            }
        }

        self.tiles.insert(id, tile);
        self.access_order.push_back(id);
    }

    /// Removes a tile from the cache.
    pub fn remove(&mut self, id: &TileId) -> Option<T> {
        self.access_order.retain(|x| x != id);
        self.tiles.remove(id)
    }

    /// Returns true if the cache contains the tile.
    pub fn contains(&self, id: &TileId) -> bool {
        self.tiles.contains_key(id)
    }

    /// Returns the number of cached tiles.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Takes evicted tiles for cleanup.
    pub fn take_evicted(&mut self) -> Vec<(TileId, T)> {
        std::mem::take(&mut self.evicted)
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.tiles.clear();
        self.access_order.clear();
    }
}

/// Tile scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum tiles to load per frame.
    pub max_loads_per_frame: usize,
    /// Maximum tiles in cache.
    pub max_cache_size: usize,
    /// Maximum tiles in loading queue.
    pub max_queue_size: usize,
    /// Whether to prioritize by distance.
    pub prioritize_by_distance: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_loads_per_frame: 4,
            max_cache_size: 512,
            max_queue_size: 256,
            prioritize_by_distance: true,
        }
    }
}

/// Tile scheduler statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct SchedulerStats {
    /// Tiles loaded this frame.
    pub loaded_this_frame: u32,
    /// Tiles in cache.
    pub cached_tiles: u32,
    /// Tiles in queue.
    pub queued_tiles: u32,
    /// Cache hits.
    pub cache_hits: u64,
    /// Cache misses.
    pub cache_misses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_id() {
        let id = TileId::new(1, 2, 3);
        assert_eq!(id.x, 1);
        assert_eq!(id.y, 2);
        assert_eq!(id.level, 3);
    }

    #[test]
    fn test_tile_priority_ordering() {
        assert!(TilePriority::Critical > TilePriority::High);
        assert!(TilePriority::High > TilePriority::Normal);
        assert!(TilePriority::Normal > TilePriority::Low);
    }

    #[test]
    fn test_load_queue_basic() {
        let mut queue = TileLoadQueue::new(10);
        assert!(queue.is_empty());

        queue.enqueue(QueuedTile {
            id: TileId::new(0, 0, 0),
            priority: TilePriority::Normal,
            frame_number: 1,
            distance: 1000.0,
        });

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_load_queue_priority() {
        let mut queue = TileLoadQueue::new(10);

        queue.enqueue(QueuedTile {
            id: TileId::new(0, 0, 0),
            priority: TilePriority::Low,
            frame_number: 1,
            distance: 100.0,
        });
        queue.enqueue(QueuedTile {
            id: TileId::new(1, 1, 1),
            priority: TilePriority::Critical,
            frame_number: 1,
            distance: 200.0,
        });
        queue.enqueue(QueuedTile {
            id: TileId::new(2, 2, 2),
            priority: TilePriority::Normal,
            frame_number: 1,
            distance: 50.0,
        });

        // Should dequeue highest priority first
        let tile = queue.dequeue().unwrap();
        assert_eq!(tile.priority, TilePriority::Critical);
    }

    #[test]
    fn test_load_queue_distance_tiebreak() {
        let mut queue = TileLoadQueue::new(10);

        queue.enqueue(QueuedTile {
            id: TileId::new(0, 0, 0),
            priority: TilePriority::Normal,
            frame_number: 1,
            distance: 1000.0,
        });
        queue.enqueue(QueuedTile {
            id: TileId::new(1, 1, 1),
            priority: TilePriority::Normal,
            frame_number: 1,
            distance: 100.0, // Closer
        });

        // Should dequeue closer tile first (same priority)
        let tile = queue.dequeue().unwrap();
        assert_eq!(tile.id, TileId::new(1, 1, 1));
    }

    #[test]
    fn test_load_queue_max_size() {
        let mut queue = TileLoadQueue::new(2);

        queue.enqueue(QueuedTile {
            id: TileId::new(0, 0, 0),
            priority: TilePriority::Low,
            frame_number: 1,
            distance: 100.0,
        });
        queue.enqueue(QueuedTile {
            id: TileId::new(1, 1, 1),
            priority: TilePriority::Normal,
            frame_number: 1,
            distance: 100.0,
        });
        queue.enqueue(QueuedTile {
            id: TileId::new(2, 2, 2),
            priority: TilePriority::High,
            frame_number: 1,
            distance: 100.0,
        });

        // Should have evicted lowest priority
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_cache_basic() {
        let mut cache = TileCache::new(10);
        assert!(cache.is_empty());

        cache.insert(TileId::new(0, 0, 0), "tile0");
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&TileId::new(0, 0, 0)));
    }

    #[test]
    fn test_cache_get() {
        let mut cache = TileCache::new(10);
        cache.insert(TileId::new(0, 0, 0), "tile0");

        assert_eq!(cache.get(&TileId::new(0, 0, 0)), Some(&"tile0"));
        assert_eq!(cache.get(&TileId::new(1, 1, 1)), None);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = TileCache::new(2);

        cache.insert(TileId::new(0, 0, 0), "tile0");
        cache.insert(TileId::new(1, 1, 1), "tile1");
        cache.insert(TileId::new(2, 2, 2), "tile2"); // Should evict tile0

        assert_eq!(cache.len(), 2);
        assert!(!cache.contains(&TileId::new(0, 0, 0)));
        assert!(cache.contains(&TileId::new(1, 1, 1)));
        assert!(cache.contains(&TileId::new(2, 2, 2)));

        let evicted = cache.take_evicted();
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].0, TileId::new(0, 0, 0));
    }

    #[test]
    fn test_cache_access_updates_lru() {
        let mut cache = TileCache::new(2);

        cache.insert(TileId::new(0, 0, 0), "tile0");
        cache.insert(TileId::new(1, 1, 1), "tile1");

        // Access tile0 to make it recently used
        cache.get(&TileId::new(0, 0, 0));

        // Insert tile2 - should evict tile1 (least recently used)
        cache.insert(TileId::new(2, 2, 2), "tile2");

        assert!(cache.contains(&TileId::new(0, 0, 0)));
        assert!(!cache.contains(&TileId::new(1, 1, 1)));
        assert!(cache.contains(&TileId::new(2, 2, 2)));
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = TileCache::new(10);
        cache.insert(TileId::new(0, 0, 0), "tile0");

        let removed = cache.remove(&TileId::new(0, 0, 0));
        assert_eq!(removed, Some("tile0"));
        assert!(!cache.contains(&TileId::new(0, 0, 0)));
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.max_loads_per_frame, 4);
        assert_eq!(config.max_cache_size, 512);
        assert_eq!(config.max_queue_size, 256);
        assert!(config.prioritize_by_distance);
    }

    #[test]
    fn test_scheduler_stats_default() {
        let stats = SchedulerStats::default();
        assert_eq!(stats.loaded_this_frame, 0);
        assert_eq!(stats.cached_tiles, 0);
        assert_eq!(stats.cache_hits, 0);
    }
}
