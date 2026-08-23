//! Ported from `packages/engine/Source/Scene/TileReplacementQueue.js`.
//!
//! A least-recently-used queue for managing tile cache eviction.

use crate::quadtree_tile::QuadtreeTile;

/// A least-recently-used queue for managing tile cache eviction.
///
/// Tiles that haven't been rendered recently are evicted first when the
/// cache exceeds the configured size. This mirrors CesiumJS's
/// TileReplacementQueue which uses a linked-list approach.
pub struct TileReplacementQueue {
    /// The tiles in the queue, ordered from least-recently-used to most-recently-used.
    tiles: Vec<QuadtreeTile>,
    /// The maximum number of tiles to keep in the cache.
    max_tiles: i32,
}

impl TileReplacementQueue {
    /// Creates a new TileReplacementQueue.
    pub fn new() -> Self {
        Self {
            tiles: Vec::new(),
            max_tiles: 100,
        }
    }

    /// Sets the maximum number of tiles to keep.
    pub fn set_max_tiles(&mut self, max_tiles: i32) {
        self.max_tiles = max_tiles;
    }

    /// Returns the number of tiles in the queue.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Returns whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Adds a tile to the most-recently-used end of the queue.
    pub fn mark_used(&mut self, tile: QuadtreeTile) {
        self.tiles.push(tile);
    }

    /// Removes and returns the least-recently-used tile, if the queue exceeds max size.
    pub fn pop_least_recently_used(&mut self) -> Option<QuadtreeTile> {
        if self.tiles.len() > self.max_tiles as usize && !self.tiles.is_empty() {
            Some(self.tiles.remove(0))
        } else {
            None
        }
    }

    /// Clears the queue.
    pub fn clear(&mut self) {
        self.tiles.clear();
    }
}

impl Default for TileReplacementQueue {
    fn default() -> Self { Self::new() }
}
