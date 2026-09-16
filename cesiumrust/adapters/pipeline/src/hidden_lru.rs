//! Hidden-entity LRU tracking for warm fallback tiles.
//!
//! Mirrors the despawn/hidden-LRU stage in `dynamic_globe.rs::process_pipeline`
//! (L1250-1322): tiles that leave the visible set are hidden (Visibility::Hidden)
//! rather than immediately despawned, so a zoom-out can re-partition onto live
//! coarse tiles instead of flashing down to the base sphere.
//!
//! When `MAX_TILE_ENTITIES` (1800) is exceeded, the least-recently-hidden
//! tiles are despawned first (within `MAX_DESPAWNS_PER_FRAME` = 24 budget).

use std::collections::HashMap;
use std::hash::Hash;

/// LRU tracker for hidden (warm fallback) tile entities.
///
/// Tiles are moved here when they leave the visible set. The monotonically
/// increasing `tick` determines eviction order (lowest tick = oldest = first
/// to despawn when over budget).
pub struct HiddenLru<K: Hash + Eq + Copy> {
    /// Map from tile key to the tick when it was hidden.
    entries: HashMap<K, u64>,
    /// Monotonic tick counter (incremented each frame).
    tick: u64,
}

impl<K: Hash + Eq + Copy> HiddenLru<K> {
    /// Create an empty LRU tracker.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            tick: 0,
        }
    }

    /// Advance the frame tick. Called once per frame before hide/show operations.
    pub fn advance_frame(&mut self) {
        self.tick += 1;
    }

    /// Mark a tile as hidden (left visible set). Records current tick for LRU order.
    ///
    /// Corresponds to L1250-1280: entity set to Visibility::Hidden, moved to
    /// the hidden warm pool.
    pub fn hide(&mut self, key: K) {
        self.entries.insert(key, self.tick);
    }

    /// Mark a tile as visible again (re-entered visible set). Removes from LRU.
    ///
    /// Corresponds to L1285-1300: hidden tile re-activated on zoom-out.
    pub fn show(&mut self, key: &K) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Check if a tile is currently hidden.
    pub fn is_hidden(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Number of hidden tiles.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if no tiles are hidden.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Pop up to `budget` least-recently-hidden keys for despawn.
    ///
    /// Corresponds to L1300-1322: despawn oldest hidden tiles within
    /// `MAX_DESPAWNS_PER_FRAME` (24) budget when over `MAX_TILE_ENTITIES`.
    pub fn pop_lru(&mut self, budget: usize) -> Vec<K> {
        if self.entries.is_empty() || budget == 0 {
            return Vec::new();
        }

        let mut sorted: Vec<(K, u64)> = self.entries.drain().collect();
        sorted.sort_by_key(|(_, tick)| *tick);

        let take = budget.min(sorted.len());
        let evicted: Vec<K> = sorted.drain(..take).map(|(k, _)| k).collect();

        // Put back the remaining entries
        self.entries.extend(sorted);

        evicted
    }
}

impl<K: Hash + Eq + Copy> Default for HiddenLru<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TileKey = (u32, u32, u32);

    #[test]
    fn hide_and_show() {
        let mut lru = HiddenLru::<TileKey>::new();
        lru.advance_frame();
        lru.hide((1, 1, 5));
        assert!(lru.is_hidden(&(1, 1, 5)));
        assert_eq!(lru.len(), 1);

        assert!(lru.show(&(1, 1, 5)));
        assert!(!lru.is_hidden(&(1, 1, 5)));
        assert_eq!(lru.len(), 0);
    }

    #[test]
    fn pop_lru_evicts_oldest_first() {
        let mut lru = HiddenLru::<TileKey>::new();

        lru.advance_frame(); // tick=1
        lru.hide((1, 0, 4));

        lru.advance_frame(); // tick=2
        lru.hide((2, 0, 4));

        lru.advance_frame(); // tick=3
        lru.hide((3, 0, 4));

        let evicted = lru.pop_lru(2);
        assert_eq!(evicted, vec![(1, 0, 4), (2, 0, 4)]);
        assert_eq!(lru.len(), 1);
        assert!(lru.is_hidden(&(3, 0, 4)));
    }

    #[test]
    fn pop_lru_respects_budget() {
        let mut lru = HiddenLru::<TileKey>::new();
        lru.advance_frame();
        for i in 0..10 {
            lru.hide((i, 0, 5));
        }
        let evicted = lru.pop_lru(3);
        assert_eq!(evicted.len(), 3);
        assert_eq!(lru.len(), 7);
    }

    #[test]
    fn show_prevents_eviction() {
        let mut lru = HiddenLru::<TileKey>::new();
        lru.advance_frame();
        lru.hide((1, 0, 4));
        lru.advance_frame();
        lru.hide((2, 0, 4));

        // Re-show the oldest
        lru.show(&(1, 0, 4));

        let evicted = lru.pop_lru(5);
        assert_eq!(evicted, vec![(2, 0, 4)]);
    }
}
