//! Tile replacement queue: LRU-based tile management.
//!
//! Maps to CesiumJS `Scene/TileReplacementQueue.js`.
//!
//! A priority queue of tiles to be replaced, if necessary, to make room for new tiles.
//! The queue is implemented as a doubly-linked list with a frame boundary marker.

use std::collections::HashMap;

/// A unique identifier for a tile in the queue.
pub type TileId = u64;

/// Internal node in the doubly-linked list.
#[derive(Debug, Clone)]
struct QueueNode {
    tile_id: TileId,
    eligible_for_unloading: bool,
    prev: Option<TileId>,
    next: Option<TileId>,
}

/// A queue that tracks tile usage for LRU-based replacement.
///
/// Tiles rendered in the current frame are moved to the head.
/// At the start of each frame, the current head is saved as a marker
/// (`last_before_start_of_frame`). During trimming, tiles from the tail
/// up to and including the marker can be removed (if eligible).
///
/// Maps to CesiumJS `TileReplacementQueue`.
#[derive(Debug)]
pub struct TileReplacementQueue {
    /// Map from tile ID to node.
    nodes: HashMap<TileId, QueueNode>,
    /// Head of the list (most recently used).
    head: Option<TileId>,
    /// Tail of the list (least recently used).
    tail: Option<TileId>,
    /// The last tile before the start of the current render frame.
    /// Tiles closer to the head than this were used in the current frame.
    last_before_start_of_frame: Option<TileId>,
    /// Number of tiles in the queue.
    count: usize,
}

impl TileReplacementQueue {
    /// Creates a new empty queue.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            head: None,
            tail: None,
            last_before_start_of_frame: None,
            count: 0,
        }
    }

    /// Returns the number of tiles in the queue.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the head tile ID (most recently rendered).
    pub fn head(&self) -> Option<TileId> {
        self.head
    }

    /// Returns the tail tile ID (least recently rendered).
    pub fn tail(&self) -> Option<TileId> {
        self.tail
    }

    /// Marks the start of a new render frame.
    ///
    /// Saves the current head as the frame boundary marker.
    /// Tiles before (closer to head) this marker were used in the current frame
    /// and must not be unloaded.
    ///
    /// Maps to `TileReplacementQueue.markStartOfRenderFrame`.
    pub fn mark_start_of_render_frame(&mut self) {
        self.last_before_start_of_frame = self.head;
    }

    /// Marks a tile as rendered in the current frame.
    ///
    /// Moves the tile to the head of the list (MRU position).
    /// If the tile is already the head and is the frame marker,
    /// the marker is advanced to the next tile.
    ///
    /// Maps to `TileReplacementQueue.markTileRendered`.
    pub fn mark_tile_rendered(&mut self, tile_id: TileId, eligible_for_unloading: bool) {
        if self.head == Some(tile_id) {
            // Already at head
            if self.last_before_start_of_frame == Some(tile_id) {
                // Advance marker to next
                let next = self.nodes[&tile_id].next;
                self.last_before_start_of_frame = next;
            }
            // Update eligibility
            if let Some(node) = self.nodes.get_mut(&tile_id) {
                node.eligible_for_unloading = eligible_for_unloading;
            }
            return;
        }

        let is_existing = self.nodes.contains_key(&tile_id);

        if is_existing {
            // Tile already in list, unlink from current position (keep in map)
            self.unlink(tile_id);
        } else {
            // New tile - insert into map
            self.count += 1;
            self.nodes.insert(
                tile_id,
                QueueNode {
                    tile_id,
                    eligible_for_unloading,
                    prev: None,
                    next: None,
                },
            );
        }

        // Insert at head
        let old_head = self.head;

        if let Some(node) = self.nodes.get_mut(&tile_id) {
            node.eligible_for_unloading = eligible_for_unloading;
            node.prev = None;
            node.next = old_head;
        }

        if let Some(old_head_id) = old_head {
            if let Some(old_head_node) = self.nodes.get_mut(&old_head_id) {
                old_head_node.prev = Some(tile_id);
            }
        }

        self.head = Some(tile_id);
        if self.tail.is_none() {
            self.tail = Some(tile_id);
        }
    }

    /// Reduces the size of the queue to a specified size by unloading the
    /// least-recently used tiles.
    ///
    /// Tiles from the tail up to and including the frame marker are processed.
    /// Eligible tiles are removed; ineligible tiles are skipped.
    /// Trimming stops after processing the marker tile.
    ///
    /// Maps to `TileReplacementQueue.trimTiles`.
    pub fn trim_tiles(&mut self, maximum_tiles: usize) {
        let mut tile_to_trim = self.tail;
        let mut keep_trimming = true;

        while keep_trimming
            && self.last_before_start_of_frame.is_some()
            && self.count > maximum_tiles
            && tile_to_trim.is_some()
        {
            let tile_id = tile_to_trim.unwrap();

            // Stop trimming after we process the last tile not used in the current frame
            keep_trimming = self.last_before_start_of_frame != Some(tile_id);

            let previous = self.nodes[&tile_id].prev;
            let eligible = self.nodes[&tile_id].eligible_for_unloading;

            if eligible {
                self.remove_node(tile_id);
            }

            tile_to_trim = previous;
        }
    }

    /// Removes a specific tile from the queue.
    pub fn remove(&mut self, tile_id: TileId) {
        if !self.nodes.contains_key(&tile_id) {
            return;
        }
        self.remove_node(tile_id);
    }

    /// Returns whether a tile is in the queue.
    pub fn contains(&self, tile_id: TileId) -> bool {
        self.nodes.contains_key(&tile_id)
    }

    // ─── Internal helpers ─────────────────────────────────────────────────────

    /// Unlinks a node from the doubly-linked list WITHOUT removing it from the map.
    /// Used when moving a tile to head. Does NOT change count.
    fn unlink(&mut self, item_id: TileId) {
        let (prev, next) = {
            let node = &self.nodes[&item_id];
            (node.prev, node.next)
        };

        // If unlinking the marker, advance marker to next
        if self.last_before_start_of_frame == Some(item_id) {
            self.last_before_start_of_frame = next;
        }

        // Update head
        if self.head == Some(item_id) {
            self.head = next;
        } else if let Some(prev_id) = prev {
            if let Some(prev_node) = self.nodes.get_mut(&prev_id) {
                prev_node.next = next;
            }
        }

        // Update tail
        if self.tail == Some(item_id) {
            self.tail = prev;
        } else if let Some(next_id) = next {
            if let Some(next_node) = self.nodes.get_mut(&next_id) {
                next_node.prev = prev;
            }
        }
    }

    /// Completely removes a node from the list AND the map. Decrements count.
    /// Faithful to CesiumJS `remove` function.
    fn remove_node(&mut self, item_id: TileId) {
        let (prev, next) = {
            let node = &self.nodes[&item_id];
            (node.prev, node.next)
        };

        // If removing the marker, advance marker to next
        if self.last_before_start_of_frame == Some(item_id) {
            self.last_before_start_of_frame = next;
        }

        // Update head
        if self.head == Some(item_id) {
            self.head = next;
        } else if let Some(prev_id) = prev {
            if let Some(prev_node) = self.nodes.get_mut(&prev_id) {
                prev_node.next = next;
            }
        }

        // Update tail
        if self.tail == Some(item_id) {
            self.tail = prev;
        } else if let Some(next_id) = next {
            if let Some(next_node) = self.nodes.get_mut(&next_id) {
                next_node.prev = prev;
            }
        }

        self.nodes.remove(&item_id);
        self.count -= 1;
    }
}

impl Default for TileReplacementQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_queue_empty() {
        let queue = TileReplacementQueue::new();
        assert_eq!(queue.count(), 0);
        assert_eq!(queue.head(), None);
        assert_eq!(queue.tail(), None);
    }

    #[test]
    fn test_mark_tile_rendered_adds() {
        let mut queue = TileReplacementQueue::new();
        queue.mark_tile_rendered(1, true);
        assert_eq!(queue.count(), 1);
        assert_eq!(queue.head(), Some(1));
        assert_eq!(queue.tail(), Some(1));
    }

    #[test]
    fn test_mark_tile_rendered_moves_to_head() {
        let mut queue = TileReplacementQueue::new();
        queue.mark_tile_rendered(1, true);
        queue.mark_tile_rendered(2, true);
        queue.mark_tile_rendered(3, true);

        // Order: 3 -> 2 -> 1
        assert_eq!(queue.head(), Some(3));
        assert_eq!(queue.tail(), Some(1));

        // Move 1 to head
        queue.mark_tile_rendered(1, true);
        assert_eq!(queue.head(), Some(1));
        assert_eq!(queue.count(), 3);
    }

    #[test]
    fn test_trim_removes_previous_frame() {
        let mut queue = TileReplacementQueue::new();
        queue.mark_tile_rendered(1, true);
        queue.mark_tile_rendered(2, true);
        queue.mark_tile_rendered(3, true);
        queue.mark_start_of_render_frame();

        queue.trim_tiles(1);
        assert_eq!(queue.count(), 1);
        assert_eq!(queue.head(), Some(3));
    }

    #[test]
    fn test_trim_skips_ineligible() {
        let mut queue = TileReplacementQueue::new();
        queue.mark_tile_rendered(1, true);
        queue.mark_tile_rendered(2, false); // Not eligible
        queue.mark_tile_rendered(3, true);
        queue.mark_start_of_render_frame();

        // marker=3, trim: 1(remove), 2(skip), 3(marker, remove, stop)
        queue.trim_tiles(0);
        assert_eq!(queue.count(), 1);
        assert_eq!(queue.head(), Some(2));
    }
}
