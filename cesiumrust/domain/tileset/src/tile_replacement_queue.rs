//! 瓦片替换队列：基于 LRU 的瓦片管理。
//!
//! 镜像 CesiumJS `Scene/TileReplacementQueue.js`。
//!
//! 一个待替换瓦片的优先队列，必要时为新瓦片腾出空间。
//! 该队列实现为带帧边界标记的双链表。

// 遗留的 CesiumJS 移植风格债务（deferred.md #18）；在 M13 lint-cleanup 或本文件在其里程碑被重写时重新审视
#![allow(dead_code)]
use std::collections::HashMap;

/// 队列中瓦片的唯一标识符。
pub type TileId = u64;

/// 双链表中的内部节点。
#[derive(Debug, Clone)]
struct QueueNode {
    tile_id: TileId,
    eligible_for_unloading: bool,
    prev: Option<TileId>,
    next: Option<TileId>,
}

/// 跟踪瓦片使用情况以实现基于 LRU 替换的队列。
///
/// 当前帧中渲染的瓦片会被移到头部。
/// 每帧开始时，当前头部被保存为标记
/// （`last_before_start_of_frame`）。修剪时，从尾部直到（并包括）
/// 该标记的瓦片可被移除（若符合条件）。
///
/// 映射到 CesiumJS `TileReplacementQueue`。
#[derive(Debug)]
pub struct TileReplacementQueue {
    /// 从瓦片 ID 到节点的映射。
    nodes: HashMap<TileId, QueueNode>,
    /// 链表头部（最近使用）。
    head: Option<TileId>,
    /// 链表尾部（最少使用）。
    tail: Option<TileId>,
    /// 当前渲染帧开始前的最后一个瓦片。
    /// 比此标记更靠近 head 的瓦片在当前帧中被使用过。
    last_before_start_of_frame: Option<TileId>,
    /// 队列中的瓦片数。
    count: usize,
}

impl TileReplacementQueue {
    /// 创建一个空队列。
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            head: None,
            tail: None,
            last_before_start_of_frame: None,
            count: 0,
        }
    }

    /// 返回队列中的瓦片数。
    pub fn count(&self) -> usize {
        self.count
    }

    /// 返回头部瓦片 ID（最近渲染的）。
    pub fn head(&self) -> Option<TileId> {
        self.head
    }

    /// 返回尾部瓦片 ID（最少渲染的）。
    pub fn tail(&self) -> Option<TileId> {
        self.tail
    }

    /// 标记一个新渲染帧的开始。
    ///
    /// 将当前 head 保存为帧边界标记。
    /// 位于此标记之前（更靠近 head）的瓦片在当前帧中被使用过，
    /// 不得卸载。
    ///
    /// 映射到 `TileReplacementQueue.markStartOfRenderFrame`。
    pub fn mark_start_of_render_frame(&mut self) {
        self.last_before_start_of_frame = self.head;
    }

    /// 标记一个瓦片在当前帧中已渲染。
    ///
    /// 将该瓦片移到链表头部（MRU 位置）。
    /// 若该瓦片已是头部且为帧标记，
    /// 则将标记推进到下一个瓦片。
    ///
    /// 映射到 `TileReplacementQueue.markTileRendered`。
    pub fn mark_tile_rendered(&mut self, tile_id: TileId, eligible_for_unloading: bool) {
        if self.head == Some(tile_id) {
            // 已在 head
            if self.last_before_start_of_frame == Some(tile_id) {
                // 将标记推进到下一个
                let next = self.nodes[&tile_id].next;
                self.last_before_start_of_frame = next;
            }
            // 更新 eligibility
            if let Some(node) = self.nodes.get_mut(&tile_id) {
                node.eligible_for_unloading = eligible_for_unloading;
            }
            return;
        }

        let is_existing = self.nodes.contains_key(&tile_id);

        if is_existing {
            // 瓦片已在链表中，从当前位置解除链接（保留在 map 中）
            self.unlink(tile_id);
        } else {
            // 新瓦片 - 插入 map
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

        // 在 head 处插入
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

    /// 通过将最少使用的瓦片卸载，将队列缩减到指定大小。
    ///
    /// 处理从尾部直到（并包括）帧标记的瓦片。
    /// 符合条件的瓦片被移除；不符合条件的被跳过。
    /// 处理完标记瓦片后停止修剪。
    ///
    /// 映射到 `TileReplacementQueue.trimTiles`。
    pub fn trim_tiles(&mut self, maximum_tiles: usize) {
        let mut tile_to_trim = self.tail;
        let mut keep_trimming = true;

        while keep_trimming
            && self.last_before_start_of_frame.is_some()
            && self.count > maximum_tiles
            && tile_to_trim.is_some()
        {
            let tile_id = tile_to_trim.unwrap();

            // 处理完当前帧未使用的最后一个瓦片后停止修剪
            keep_trimming = self.last_before_start_of_frame != Some(tile_id);

            let previous = self.nodes[&tile_id].prev;
            let eligible = self.nodes[&tile_id].eligible_for_unloading;

            if eligible {
                self.remove_node(tile_id);
            }

            tile_to_trim = previous;
        }
    }

    /// 从队列中移除特定瓦片。
    pub fn remove(&mut self, tile_id: TileId) {
        if !self.nodes.contains_key(&tile_id) {
            return;
        }
        self.remove_node(tile_id);
    }

    /// 返回某瓦片是否在队列中。
    pub fn contains(&self, tile_id: TileId) -> bool {
        self.nodes.contains_key(&tile_id)
    }

    // ─── 内部辅助函数 ─────────────────────────────────────────────────────

    /// 将节点从双链表中解除链接，但不从 map 中移除。
    /// 用于将瓦片移到 head 时。不改变 count。
    fn unlink(&mut self, item_id: TileId) {
        let (prev, next) = {
            let node = &self.nodes[&item_id];
            (node.prev, node.next)
        };

        // 若正在解除链接的是标记，将标记推进到下一个
        if self.last_before_start_of_frame == Some(item_id) {
            self.last_before_start_of_frame = next;
        }

        // 更新 head
        if self.head == Some(item_id) {
            self.head = next;
        } else if let Some(prev_id) = prev {
            if let Some(prev_node) = self.nodes.get_mut(&prev_id) {
                prev_node.next = next;
            }
        }

        // 更新 tail
        if self.tail == Some(item_id) {
            self.tail = prev;
        } else if let Some(next_id) = next {
            if let Some(next_node) = self.nodes.get_mut(&next_id) {
                next_node.prev = prev;
            }
        }
    }

    /// 将节点从链表和 map 中完全移除。递减 count。
    /// 忠于 CesiumJS 的 `remove` 函数。
    fn remove_node(&mut self, item_id: TileId) {
        let (prev, next) = {
            let node = &self.nodes[&item_id];
            (node.prev, node.next)
        };

        // 若正在移除的是标记，将标记推进到下一个
        if self.last_before_start_of_frame == Some(item_id) {
            self.last_before_start_of_frame = next;
        }

        // 更新 head
        if self.head == Some(item_id) {
            self.head = next;
        } else if let Some(prev_id) = prev {
            if let Some(prev_node) = self.nodes.get_mut(&prev_id) {
                prev_node.next = next;
            }
        }

        // 更新 tail
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

        // 顺序：3 -> 2 -> 1
        assert_eq!(queue.head(), Some(3));
        assert_eq!(queue.tail(), Some(1));

        // 将 1 移到 head
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
        queue.mark_tile_rendered(2, false); // 不符合条件
        queue.mark_tile_rendered(3, true);
        queue.mark_start_of_render_frame();

        // marker=3，trim：1(移除), 2(跳过), 3(marker, 移除, 停止)
        queue.trim_tiles(0);
        assert_eq!(queue.count(), 1);
        assert_eq!(queue.head(), Some(2));
    }
}
