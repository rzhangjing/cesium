//! TileReplacementQueue specs - LRU-based tile management
//! Ported from Scene/TileReplacementQueueSpec.js (7 A-class tests)

use cesium_tileset::tile_replacement_queue::TileReplacementQueue;

// ─── markStartOfRenderFrame ─────────────────────────────────────────────────

#[test]
fn prevents_tiles_added_afterward_from_being_trimmed() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_start_of_render_frame();

    queue.mark_tile_rendered(3, true);

    queue.trim_tiles(0);

    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(3));
}

#[test]
fn prevents_all_tiles_from_being_trimmed_if_called_on_empty_queue() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_start_of_render_frame();

    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);

    queue.trim_tiles(0);
    assert_eq!(queue.count(), 3);
}

#[test]
fn adjusts_properly_when_last_tile_in_previous_frame_moved_to_head() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);

    queue.mark_start_of_render_frame();

    queue.mark_tile_rendered(3, true);

    queue.trim_tiles(0);
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(3));
}

#[test]
fn adjusts_properly_when_all_tiles_moved_to_head() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);

    queue.mark_start_of_render_frame();

    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);

    queue.trim_tiles(0);
    assert_eq!(queue.count(), 3);
    assert_eq!(queue.head(), Some(3));
    assert_eq!(queue.tail(), Some(1));
}

// ─── trimTiles ──────────────────────────────────────────────────────────────

#[test]
fn does_not_remove_tile_not_eligible_for_unloading() {
    // Ported from CesiumJS: markTileRendered(one, two, notEligible, three)
    // After trim, only notEligible remains (ineligible tiles are skipped, not blockers)
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(99, false); // not eligible
    queue.mark_tile_rendered(3, true);

    queue.mark_start_of_render_frame();

    queue.trim_tiles(0);
    // Tiles 1, 2 removed (eligible), 99 skipped (not eligible), 3 removed (marker, eligible)
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(99));
}

#[test]
fn does_not_remove_transitioning_tile_at_end_of_last_render_frame() {
    // Ported from CesiumJS: notEligible is the marker (head at markStartOfRenderFrame)
    // Trimming processes all tiles up to marker; marker itself is not eligible → kept
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);
    queue.mark_tile_rendered(99, false); // not eligible, becomes marker

    queue.mark_start_of_render_frame();

    queue.trim_tiles(0);
    // 1, 2, 3 removed (eligible); 99 is marker + not eligible → kept, trimming stops
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(99));
}

#[test]
fn removes_two_tiles_not_used_last_render_frame() {
    // Ported from CesiumJS: notEligible at tail, one/two in middle, three/four current frame
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(99, false); // not eligible, at tail
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true); // marker after markStartOfRenderFrame
    queue.mark_start_of_render_frame();
    queue.mark_tile_rendered(3, true);
    queue.mark_tile_rendered(4, true);
    queue.trim_tiles(0);
    // From tail: 99 skipped (not eligible), 1 removed (eligible),
    // 2 is marker (eligible → removed, marker advances, stop)
    // Remaining: 99 (skipped), 3, 4 (current frame)
    assert_eq!(queue.count(), 3);
    assert!(queue.contains(3));
    assert!(queue.contains(4));
    assert!(queue.contains(99));
    assert!(!queue.contains(1));
    assert!(!queue.contains(2));
}

// ─── Additional edge cases ──────────────────────────────────────────────────

#[test]
fn new_queue_is_empty() {
    let queue = TileReplacementQueue::new();
    assert_eq!(queue.count(), 0);
    assert_eq!(queue.head(), None);
    assert_eq!(queue.tail(), None);
}

#[test]
fn mark_rendered_single_tile() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(42, true);
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(42));
    assert_eq!(queue.tail(), Some(42));
}

#[test]
fn mark_rendered_moves_existing_to_head() {
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
    assert_eq!(queue.tail(), Some(2));
    assert_eq!(queue.count(), 3);
}

#[test]
fn remove_specific_tile() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);

    queue.remove(2);
    assert_eq!(queue.count(), 2);
    assert!(!queue.contains(2));
    assert!(queue.contains(1));
    assert!(queue.contains(3));
}

#[test]
fn remove_head_tile() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);

    queue.remove(2); // head
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(1));
    assert_eq!(queue.tail(), Some(1));
}

#[test]
fn remove_tail_tile() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);

    queue.remove(1); // tail
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(2));
    assert_eq!(queue.tail(), Some(2));
}

#[test]
fn trim_with_maximum_tiles() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);
    queue.mark_tile_rendered(4, true);
    queue.mark_tile_rendered(5, true);
    queue.mark_start_of_render_frame();
    // marker = 5 (head), no current-frame tiles added
    // trim from tail: 1,2,3,4 eligible→removed; 5 is marker, eligible→removed, stop
    // But maximum_tiles=3, so trimming stops when count<=3
    queue.trim_tiles(3);
    // Trim: 1(count5→4), 2(count4→3, count<=3 stop)
    assert_eq!(queue.count(), 3);
    assert!(queue.contains(5));
    assert!(queue.contains(4));
    assert!(queue.contains(3));
}

#[test]
fn contains_returns_false_for_removed() {
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    assert!(queue.contains(1));

    queue.remove(1);
    assert!(!queue.contains(1));
}
