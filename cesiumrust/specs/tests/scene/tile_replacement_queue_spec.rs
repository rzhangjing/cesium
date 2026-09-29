//! TileReplacementQueue specs - 基于 LRU 的瓦片管理
//! 移植自 Scene/TileReplacementQueueSpec.js（7 个 A 类测试）

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
    // 移植自 CesiumJS： markTileRendered(one, two, notEligible, three)
    // 裁剪后仅 notEligible 保留（不合格的瓦片被跳过，并非阻塞项）
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(99, false); // 不合格
    queue.mark_tile_rendered(3, true);

    queue.mark_start_of_render_frame();

    queue.trim_tiles(0);
    // 瓦片 1, 2 被移除（合格），99 被跳过（不合格），3 被移除（marker，合格）
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(99));
}

#[test]
fn does_not_remove_transitioning_tile_at_end_of_last_render_frame() {
    // 移植自 CesiumJS： notEligible is the marker (head at markStartOfRenderFrame)
    // 裁剪处理到 marker 为止的所有瓦片；marker 本身不合格 → 保留
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true);
    queue.mark_tile_rendered(3, true);
    queue.mark_tile_rendered(99, false); // 不合格，成为 marker

    queue.mark_start_of_render_frame();

    queue.trim_tiles(0);
    // 1, 2, 3 被移除（合格）；99 是 marker + 不合格 → 保留，裁剪停止
    assert_eq!(queue.count(), 1);
    assert_eq!(queue.head(), Some(99));
}

#[test]
fn removes_two_tiles_not_used_last_render_frame() {
    // 移植自 CesiumJS： notEligible at tail, one/two in middle, three/four current frame
    let mut queue = TileReplacementQueue::new();
    queue.mark_tile_rendered(99, false); // 不合格，在 tail
    queue.mark_tile_rendered(1, true);
    queue.mark_tile_rendered(2, true); // markStartOfRenderFrame 之后的 marker
    queue.mark_start_of_render_frame();
    queue.mark_tile_rendered(3, true);
    queue.mark_tile_rendered(4, true);
    queue.trim_tiles(0);
    // 从 tail：99 被跳过（不合格），1 被移除（合格），
    // 2 是 marker（合格 → 移除，marker 前移，停止）
    // 剩余：99（跳过），3, 4（当前帧）
    assert_eq!(queue.count(), 3);
    assert!(queue.contains(3));
    assert!(queue.contains(4));
    assert!(queue.contains(99));
    assert!(!queue.contains(1));
    assert!(!queue.contains(2));
}

// ─── 额外边缘情形 ──────────────────────────────────────────────────

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

    // 顺序：3 -> 2 -> 1
    assert_eq!(queue.head(), Some(3));
    assert_eq!(queue.tail(), Some(1));

    // 将 1 移到 head
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
    // marker = 5（head），未添加当前帧瓦片
    // 从 tail 裁剪：1,2,3,4 合格→移除；5 是 marker，合格→移除，停止
    // 但 maximum_tiles=3，因此当 count<=3 时裁剪停止
    queue.trim_tiles(3);
    // 裁剪：1(count5→4), 2(count4→3, count<=3 停止)
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
