//! In-flight deduplication set.
//!
//! Mirrors `dynamic_globe.rs` dedup logic in `enqueue_tiles` (L406, L417):
//! a tile already in `in_flight` or `queued` is never re-submitted, preventing
//! duplicate downloads and redundant mesh builds.

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Mutex;

/// Thread-safe deduplication tracker for in-flight tile requests.
///
/// Corresponds to `TileManager::in_flight: HashSet<TileKey>` and
/// `TileManager::queued: HashSet<TileKey>` in dynamic_globe.rs.
pub struct Dedup<K: Hash + Eq + Copy> {
    inner: Mutex<HashSet<K>>,
}

impl<K: Hash + Eq + Copy> Dedup<K> {
    /// Create an empty dedup set.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashSet::new()),
        }
    }

    /// Try to insert a key. Returns `true` if newly inserted (not a duplicate),
    /// `false` if already present (deduplicated — skip submission).
    ///
    /// Corresponds to L406: `if mgr.tile_entities.contains_key(&key) || mgr.queued.contains(&key) { continue; }`
    /// and L417: `!mgr.in_flight.contains(&key)`.
    pub fn insert(&self, key: K) -> bool {
        self.inner.lock().unwrap().insert(key)
    }

    /// Remove a key (tile completed or cancelled).
    ///
    /// Corresponds to L1051: `mgr.in_flight.remove(&key)`.
    pub fn remove(&self, key: &K) -> bool {
        self.inner.lock().unwrap().remove(key)
    }

    /// Check if a key is currently in-flight.
    pub fn contains(&self, key: &K) -> bool {
        self.inner.lock().unwrap().contains(key)
    }

    /// Current number of in-flight keys.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// Returns true if no keys are in-flight.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries (e.g. on pipeline reset).
    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

impl<K: Hash + Eq + Copy> Default for Dedup<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TileKey = (u32, u32, u32);

    #[test]
    fn insert_new_key_returns_true() {
        let d = Dedup::<TileKey>::new();
        assert!(d.insert((1, 2, 3)));
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn insert_duplicate_returns_false() {
        let d = Dedup::<TileKey>::new();
        assert!(d.insert((1, 2, 3)));
        assert!(!d.insert((1, 2, 3)));
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn remove_allows_reinsert() {
        let d = Dedup::<TileKey>::new();
        d.insert((4, 5, 6));
        assert!(d.remove(&(4, 5, 6)));
        assert!(d.insert((4, 5, 6)));
    }

    #[test]
    fn contains_check() {
        let d = Dedup::<TileKey>::new();
        d.insert((7, 8, 9));
        assert!(d.contains(&(7, 8, 9)));
        assert!(!d.contains(&(0, 0, 0)));
    }

    #[test]
    fn clear_empties_set() {
        let d = Dedup::<TileKey>::new();
        d.insert((1, 1, 1));
        d.insert((2, 2, 2));
        d.clear();
        assert!(d.is_empty());
    }
}
