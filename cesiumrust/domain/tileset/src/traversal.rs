//! 高级 3D Tiles 遍历策略。
//!
//! 提供基础、跳过（skip）与最详细三种遍历策略，配合内存
//! 调整的屏幕空间误差与优先级排序，输出选中瓦片与加载请求。

// 遗留的 原实现 移植风格债务（deferred.md #18）；在 M13 lint-cleanup 或本文件在其里程碑被重写时重新审视
#![allow(clippy::field_reassign_with_default)]
use crate::lod_selection::{CameraState, LodSelectionContext, SelectedTile, TileSelectionResult};
use crate::tile::{Tile, TileRefine};
use cesium_geospatial::ellipsoid::Ellipsoid;

/// 遍历策略选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraversalStrategy {
    /// 基础遍历：简单的自上而下、基于 SSE 的细化。
    #[default]
    Base,
    /// 跳过遍历：允许跳级，同时渲染父瓦片与子瓦片。
    Skip,
    /// 最详细遍历：总是细化到可用的最深内容。
    MostDetailed,
}

/// 瓦片加载请求的优先级。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TilePriority {
    /// 到相机的距离（越小优先级越高）。
    pub distance: f64,
    /// 在树中的深度（越小则祖先优先级越高）。
    pub depth: u32,
    /// 是否为某个选中瓦片的祖先。
    pub is_ancestor: bool,
}

impl TilePriority {
    /// 计算一个数值优先级（越小越先加载）。
    pub fn value(&self) -> f64 {
        // 祖先获得最高优先级（先加载父瓦片再加载子瓦片）
        let ancestor_bonus = if self.is_ancestor { -1000.0 } else { 0.0 };
        ancestor_bonus + self.distance + (self.depth as f64) * 0.01
    }
}

impl PartialOrd for TilePriority {
    /// 按数值优先级比较（委托给 [`Ord::cmp`]）。
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TilePriority {
    /// 按计算出的数值优先级排序，小者优先。
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value()
            .partial_cmp(&other.value())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Eq for TilePriority {}

/// 带优先级的瓦片请求。
#[derive(Debug, Clone)]
pub struct TileRequest {
    /// 瓦片在树中的路径。
    pub path: Vec<usize>,
    /// 加载优先级。
    pub priority: TilePriority,
}

/// 内存调整的屏幕空间误差计算。
#[derive(Debug, Clone)]
pub struct MemoryAdjustedSse {
    /// 基准最大屏幕空间误差。
    pub base_sse: f64,
    /// 最大内存（字节）。
    pub max_memory_bytes: u64,
    /// 当前内存使用量（字节）。
    pub current_memory_bytes: u64,
}

impl MemoryAdjustedSse {
    /// 创建一个新的内存调整 SSE 计算器。
    pub fn new(base_sse: f64, max_memory_bytes: u64) -> Self {
        Self {
            base_sse,
            max_memory_bytes,
            current_memory_bytes: 0,
        }
    }

    /// 计算内存调整后的 SSE 阈值。
    ///
    /// 当内存使用超过上限时，提高 SSE 阈值
    /// 以降低细节并释放内存。
    pub fn adjusted_sse(&self) -> f64 {
        if self.max_memory_bytes == 0 {
            return self.base_sse;
        }

        let usage_ratio =
            self.current_memory_bytes as f64 / self.max_memory_bytes as f64;

        if usage_ratio <= 0.5 {
            // 内存低于 50%：使用基准 SSE
            self.base_sse
        } else if usage_ratio < 1.0 {
            // 50-100%：线性提高 SSE
            let t = (usage_ratio - 0.5) / 0.5;
            self.base_sse * (1.0 + t)
        } else {
            // 超过 100%：激进地提高 SSE
            let overage = usage_ratio - 1.0;
            self.base_sse * (2.0 + overage * 4.0)
        }
    }

    /// 若内存超过上限则返回 true。
    pub fn is_over_limit(&self) -> bool {
        self.current_memory_bytes > self.max_memory_bytes
    }
}

/// 包含所有配置的遍历上下文。
#[derive(Debug, Clone)]
pub struct TraversalContext {
    /// LOD 选择上下文。
    pub lod_context: LodSelectionContext,
    /// 遍历策略。
    pub strategy: TraversalStrategy,
    /// 内存调整的 SSE。
    pub memory_sse: MemoryAdjustedSse,
    /// 每帧最多访问的瓦片数（0 = 无限）。
    pub max_tiles_per_frame: usize,
    /// 是否预加载祖先。
    pub preload_ancestors: bool,
    /// 加载后代数限制。
    pub loading_descendant_limit: u32,
}

impl Default for TraversalContext {
    /// 默认：Base 策略、预加载祖先、内存上限 512 MB、每帧不限。
    fn default() -> Self {
        Self {
            lod_context: LodSelectionContext::default(),
            strategy: TraversalStrategy::Base,
            memory_sse: MemoryAdjustedSse::new(16.0, 512 * 1024 * 1024),
            max_tiles_per_frame: 0,
            preload_ancestors: true,
            loading_descendant_limit: 20,
        }
    }
}

/// 一次遍历操作的结果。
#[derive(Debug, Clone, Default)]
pub struct TraversalResult {
    /// 选中渲染的瓦片。
    pub selected_tiles: Vec<SelectedTile>,
    /// 请求加载的瓦片（带优先级）。
    pub requested_tiles: Vec<TileRequest>,
    /// 访问的瓦片数。
    pub visited_count: usize,
    /// 被剔除的瓦片数。
    pub culled_count: usize,
    /// 达到的最大深度。
    pub max_depth: u32,
}

/// 使用配置的策略执行瓦片遍历。
pub fn traverse(
    root: &Tile,
    camera: &CameraState,
    context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    match context.strategy {
        // Base：标准 SSE 细化选择
        TraversalStrategy::Base => traverse_base(root, camera, context, ellipsoid),
        // Skip：允许跳级、父兄共渲
        TraversalStrategy::Skip => traverse_skip(root, camera, context, ellipsoid),
        // MostDetailed：强制下钻至最深内容
        TraversalStrategy::MostDetailed => {
            traverse_most_detailed(root, camera, context, ellipsoid)
        }
    }
}

/// 基础遍历：简单的自上而下、基于 SSE 的细化。
fn traverse_base(
    root: &Tile,
    camera: &CameraState,
    context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    let mut result = TraversalResult::default();
    // 取内存调整后的有效 SSE 作为本次选择的阈值
    let effective_sse = context.memory_sse.adjusted_sse();

    let mut ctx = context.lod_context.clone();
    // 覆盖 LOD 上下文的 SSE 上限为有效值
    ctx.maximum_screen_space_error = effective_sse;

    result.selected_tiles =
        crate::lod_selection::select_tiles(root, camera, &ctx, ellipsoid);
    // 基础遍历不区分访问与选中：选中数即访问数
    result.visited_count = result.selected_tiles.len();

    // 为选中的瓦片生成加载请求
    for tile in &result.selected_tiles {
        result.requested_tiles.push(TileRequest {
            path: tile.path.clone(),
            priority: TilePriority {
                distance: tile.distance_to_camera,
                depth: tile.path.len() as u32,
                is_ancestor: false,
            },
        });
    }

    result
}

/// 跳过遍历：允许跳过树中的某些层级。
///
/// 与基础遍历的关键区别：
/// - 可同时渲染父瓦片与子瓦片
/// - 子瓦片尚未加载时跳过中间层级
/// - 使用为 2 的后代选择深度
fn traverse_skip(
    root: &Tile,
    camera: &CameraState,
    context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    let mut result = TraversalResult::default();
    // 取内存调整后的有效 SSE 阈值
    let effective_sse = context.memory_sse.adjusted_sse();

    // 从根开始递归，初始父细化为 Replace、空路径、深度 0
    traverse_skip_recursive(
        root,
        camera,
        effective_sse,
        ellipsoid,
        TileRefine::Replace,
        &[],
        0,
        context.preload_ancestors,
        &mut result,
    );

    // 按优先级排序请求（祖先/近者靠先）
    result.requested_tiles.sort_by_key(|a| a.priority);

    result
}

/// 跳过遍历的递归辅助函数。
#[allow(clippy::too_many_arguments)]
fn traverse_skip_recursive(
    tile: &Tile,
    camera: &CameraState,
    max_sse: f64,
    ellipsoid: &Ellipsoid,
    parent_refine: TileRefine,
    path: &[usize],
    depth: u32,
    preload_ancestors: bool,
    result: &mut TraversalResult,
) {
    result.visited_count += 1;
    result.max_depth = result.max_depth.max(depth);

    // 计算到相机的距离与本瓦片在当前距离下的 SSE
    let distance = tile.bounding_volume.distance_to(camera.position, ellipsoid);
    let sse = camera.compute_screen_space_error(tile.geometric_error, distance);
    // 细化模式：本瓦片未声明时继承父级
    let refine_mode = tile.effective_refine(parent_refine);
    let has_children = !tile.children.is_empty();

    // 检查是否应细化
    let should_refine = has_children && sse > max_sse;

    if !should_refine {
        // 渲染本瓦片
        if tile.has_content() {
            result.selected_tiles.push(SelectedTile {
                path: path.to_vec(),
                result: TileSelectionResult::Render,
                screen_space_error: sse,
                distance_to_camera: distance,
            });
            result.requested_tiles.push(TileRequest {
                path: path.to_vec(),
                priority: TilePriority {
                    distance,
                    depth,
                    is_ancestor: false,
                },
            });
        } else if has_children {
            // 空瓦片：必须细化
            for (i, child) in tile.children.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(i);
                traverse_skip_recursive(
                    child,
                    camera,
                    max_sse,
                    ellipsoid,
                    refine_mode,
                    &child_path,
                    depth + 1,
                    preload_ancestors,
                    result,
                );
            }
        }
        return;
    }

    // 应细化：检查子瓦片是否就绪
    // 在跳过遍历中，若子瓦片未就绪则渲染父瓦片，
    // 并尝试加载子瓦片（必要时跳级）

    // 对于 ADD 细化，总是渲染父瓦片
    if refine_mode == TileRefine::Add && tile.has_content() {
        result.selected_tiles.push(SelectedTile {
            path: path.to_vec(),
            result: TileSelectionResult::Render,
            screen_space_error: sse,
            distance_to_camera: distance,
        });
    }

    // 带跳过逻辑地遍历子瓦片
    // 跳过遍历：向前看 2 层（descendantSelectionDepth = 2）
    let mut any_child_rendered = false;
    // 逐个处理子瓦片，按需跳级到孙瓦片
    for (i, child) in tile.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(i);

        // 先算子瓦片距离与 SSE，判断是否需要继续下钻
        let child_distance =
            child.bounding_volume.distance_to(camera.position, ellipsoid);
        let child_sse =
            camera.compute_screen_space_error(child.geometric_error, child_distance);

        // 若子瓦片 SSE 仍太高且有孙瓦片，则跳到孙瓦片
        if !child.children.is_empty() && child_sse > max_sse {
            // 跳级：将子瓦片作为祖先渲染，遍历孙瓦片
            if preload_ancestors && child.has_content() {
                result.requested_tiles.push(TileRequest {
                    path: child_path.clone(),
                    priority: TilePriority {
                        distance: child_distance,
                        depth: depth + 1,
                        is_ancestor: true,
                    },
                });
            }

            for (j, grandchild) in child.children.iter().enumerate() {
                // 直接以深度+2 递归到孙瓦片，跳过中间的子瓦片细化
                let mut gc_path = child_path.clone();
                gc_path.push(j);
                traverse_skip_recursive(
                    grandchild,
                    camera,
                    max_sse,
                    ellipsoid,
                    refine_mode,
                    &gc_path,
                    depth + 2,
                    preload_ancestors,
                    result,
                );
            }
            any_child_rendered = true;
        } else {
            // 对该子瓦片进行正常遍历
            traverse_skip_recursive(
                child,
                camera,
                max_sse,
                ellipsoid,
                refine_mode,
                &child_path,
                depth + 1,
                preload_ancestors,
                result,
            );
            any_child_rendered = true;
        }
    }

    // 若没有子瓦片被渲染且本瓦片有内容，将其作为后备渲染
    if !any_child_rendered && tile.has_content() && refine_mode == TileRefine::Replace {
        result.selected_tiles.push(SelectedTile {
            path: path.to_vec(),
            result: TileSelectionResult::Render,
            screen_space_error: sse,
            distance_to_camera: distance,
        });
    }
}

/// 最详细遍历：总是细化到可用的最深内容。
///
/// 该遍历用于拾取及其他需要最详细瓦片（无论 SSE 如何）的操作。
fn traverse_most_detailed(
    root: &Tile,
    camera: &CameraState,
    _context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    let mut result = TraversalResult::default();

    // 最详细策略忽略 SSE 与内存预算，无需读取 context 参数
    traverse_most_detailed_recursive(
        root,
        camera,
        ellipsoid,
        TileRefine::Replace,
        &[],
        0,
        &mut result,
    );

    result
}

/// 最详细遍历的递归辅助函数。
fn traverse_most_detailed_recursive(
    tile: &Tile,
    camera: &CameraState,
    ellipsoid: &Ellipsoid,
    parent_refine: TileRefine,
    path: &[usize],
    depth: u32,
    result: &mut TraversalResult,
) {
    result.visited_count += 1;
    result.max_depth = result.max_depth.max(depth);

    let distance = tile.bounding_volume.distance_to(camera.position, ellipsoid);
    let sse = camera.compute_screen_space_error(tile.geometric_error, distance);
    let refine_mode = tile.effective_refine(parent_refine);
    let has_children = !tile.children.is_empty();

    // 总是尝试细化到子瓦片（最详细）
    if has_children {
        // 对于 ADD 细化，同时也渲染父瓦片
        if refine_mode == TileRefine::Add && tile.has_content() {
            result.selected_tiles.push(SelectedTile {
                path: path.to_vec(),
                result: TileSelectionResult::Render,
                screen_space_error: sse,
                distance_to_camera: distance,
            });
        }

        // 逐个递归下钻到子瓦片（无视 SSE 阈值）
        for (i, child) in tile.children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(i);
            traverse_most_detailed_recursive(
                child,
                camera,
                ellipsoid,
                refine_mode,
                &child_path,
                depth + 1,
                result,
            );
        }
    } else if tile.has_content() {
        // 带内容的叶瓦片：渲染它
        result.selected_tiles.push(SelectedTile {
            path: path.to_vec(),
            result: TileSelectionResult::Render,
            screen_space_error: sse,
            distance_to_camera: distance,
        });
        // 同时登记一个非祖先的加载请求
        result.requested_tiles.push(TileRequest {
            path: path.to_vec(),
            priority: TilePriority {
                distance,
                depth,
                is_ancestor: false,
            },
        });
    }
}

/// 按到相机的距离对子瓦片排序（最远优先，用于基于栈的遍历）。
pub fn sort_children_by_distance(
    children: &[(usize, &Tile)],
    camera: &CameraState,
    ellipsoid: &Ellipsoid,
) -> Vec<usize> {
    // 先计算每个子瓦片到相机的距离，保留其原始索引
    let mut indexed: Vec<(usize, f64)> = children
        .iter()
        .map(|(i, tile)| {
            let dist = tile.bounding_volume.distance_to(camera.position, ellipsoid);
            (*i, dist)
        })
        .collect();

    // 按距离降序排序（最远优先）
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.into_iter().map(|(i, _)| i).collect()
}

/// 检查瓦片是否可被遍历（有子瓦片且 SSE 超过阈值）。
pub fn can_traverse(
    tile: &Tile,
    sse: f64,
    max_sse: f64,
    has_implicit_content: bool,
) -> bool {
    // 无子且无隐式内容则无可细化对象，直接不可遍历
    if tile.children.is_empty() && !has_implicit_content {
        return false;
    }
    // 仅当 SSE 超过阈值才需要继续细化
    sse > max_sse
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounding_volume::BoundingVolume;
    use crate::tile::TileContent;
    use glam::DVec3;

    /// 构造测试相机：位于 z=1000 朝 -z 看，45° 张角、视口高 1080。
    fn create_camera() -> CameraState {
        CameraState::new(
            DVec3::new(0.0, 0.0, 1000.0),
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            std::f64::consts::FRAC_PI_4,
            1080.0,
        )
    }

    /// 构造带球体包围与 REPLACE 细化的内部瓦片（含子瓦片列表）。
    fn create_tile(geometric_error: f64, uri: &str, children: Vec<Tile>) -> Tile {
        Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 100.0),
            geometric_error,
            refine: Some(TileRefine::Replace),
            transform: None,
            content: Some(TileContent {
                uri: uri.to_string(),
                bounding_volume: None,
                group: None,
            }),
            contents: None,
            children,
            viewer_request_volume: None,
            extras: None,
        }
    }

    /// 构造无子瓦片的叶瓦片（内部委托 create_tile 传空子列表）。
    fn create_leaf_tile(geometric_error: f64, uri: &str) -> Tile {
        create_tile(geometric_error, uri, vec![])
    }

/// 默认遍历策略应为 Base（基础的自上而下 SSE 细化）。
#[test]
    fn test_traversal_strategy_default() {
        assert_eq!(TraversalStrategy::default(), TraversalStrategy::Base);
    }

/// 祖先瓦片即便距离更远，也应比叶子瓦片拥有更高加载优先级（数值更小）。
#[test]
    fn test_tile_priority_ordering() {
        let p1 = TilePriority {
            distance: 100.0,
            depth: 0,
            is_ancestor: true,
        };
        let p2 = TilePriority {
            distance: 50.0,
            depth: 1,
            is_ancestor: false,
        };
        // 祖先应有更高的优先级（更小的值）
        assert!(p1.value() < p2.value());
    }

/// 内存远低于上限时，SSE 阈值保持基准值不变。
#[test]
    fn test_memory_adjusted_sse_under_limit() {
        let mas = MemoryAdjustedSse::new(16.0, 1000);
        assert_eq!(mas.adjusted_sse(), 16.0);
    }

/// 恰好在 50% 使用率的边界上仍使用基准 SSE（阈值为闭区间上界）。
#[test]
    fn test_memory_adjusted_sse_half_usage() {
        let mut mas = MemoryAdjustedSse::new(16.0, 1000);
        mas.current_memory_bytes = 500; // 50%
        assert_eq!(mas.adjusted_sse(), 16.0);
    }

/// 50-100% 区间线性提高 SSE：应高于基准但低于两倍基准。
#[test]
    fn test_memory_adjusted_sse_high_usage() {
        let mut mas = MemoryAdjustedSse::new(16.0, 1000);
        mas.current_memory_bytes = 750; // 75%
        let sse = mas.adjusted_sse();
        assert!(sse > 16.0);
        assert!(sse < 32.0);
    }

/// 超过 100% 时激进提高 SSE，并报告已越限。
#[test]
    fn test_memory_adjusted_sse_over_limit() {
        let mut mas = MemoryAdjustedSse::new(16.0, 1000);
        mas.current_memory_bytes = 1500; // 150%
        let sse = mas.adjusted_sse();
        assert!(sse > 32.0);
        assert!(mas.is_over_limit());
    }

/// 基础遍历应能选中至少一个瓦片（根几何误差大于 SSE 阈值时细化到子瓦片）。
#[test]
    fn test_base_traversal() {
        // 根几何误差 1000，两个子叶瓦片误差 10
        let root = create_tile(
            1000.0,
            "root.b3dm",
            vec![
                create_leaf_tile(10.0, "child0.b3dm"),
                create_leaf_tile(10.0, "child1.b3dm"),
            ],
        );
        let camera = create_camera();
        // 使用默认 Base 策略与内存调整 SSE
        let context = TraversalContext::default();

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);
        // 根误差远大于阈值，应细化并选中子叶瓦片
        assert!(!result.selected_tiles.is_empty());
    }

/// 跳过遍历允许跳级，应访问多个层级并选中非空瓦片集。
#[test]
    fn test_skip_traversal() {
        // 创建一个 3 层树：根(1000)->子(100)->孙(1)
        let grandchild = create_leaf_tile(1.0, "gc.b3dm");
        let child = create_tile(100.0, "child.b3dm", vec![grandchild]);
        let root = create_tile(1000.0, "root.b3dm", vec![child]);

        let camera = create_camera();
        let mut context = TraversalContext::default();
        // 切换为 Skip 策略以启用跳级逻辑
        context.strategy = TraversalStrategy::Skip;

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);
        assert!(!result.selected_tiles.is_empty());
        // 跳过遍历应访问了多个层级
        assert!(result.visited_count > 0);
    }

/// 最详细遍历无视 SSE 一直细化到最深叶瓦片，应选中路径 [0,0] 且最大深度为 2。
#[test]
    fn test_most_detailed_traversal() {
        // 创建一个 3 层树：最深层误差为 0（表示可用最深内容）
        let grandchild = create_leaf_tile(0.0, "gc.b3dm");
        let child = create_tile(50.0, "child.b3dm", vec![grandchild]);
        let root = create_tile(1000.0, "root.b3dm", vec![child]);

        let camera = create_camera();
        let mut context = TraversalContext::default();
        // 切换为 MostDetailed 策略：无视 SSE 一直下钻
        context.strategy = TraversalStrategy::MostDetailed;

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);

        // 最详细遍历应选出最深的瓦片（孙瓦片）
        assert!(result.selected_tiles.iter().any(|t| t.path == vec![0, 0]));
        assert_eq!(result.max_depth, 2);
    }

/// ADD 细化下最详细遍历应同时选中父瓦片与子瓦片（两者叠加渲染）。
#[test]
    fn test_most_detailed_add_refinement() {
        // 子叶瓦片误差 0，根误差 100
        let child = create_leaf_tile(0.0, "child.b3dm");
        let mut root = create_tile(100.0, "root.b3dm", vec![child]);
        // 手动改为 ADD 细化（默认创建为 REPLACE）
        root.refine = Some(TileRefine::Add);

        let camera = create_camera();
        let mut context = TraversalContext::default();
        // 策略：最详细下钻
        context.strategy = TraversalStrategy::MostDetailed;

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);

        // ADD 细化：父瓦片和子瓦片都应渲染
        assert!(result.selected_tiles.iter().any(|t| t.path.is_empty()));
        assert!(result.selected_tiles.iter().any(|t| t.path == vec![0]));
    }

/// 按距离降序排序子瓦片：离相机更远者在前（供栈式遍历优先压入近者）。
#[test]
    fn test_sort_children_by_distance() {
        // child0 位于原点（离相机 z=1000 约 1000）
        let child0 = Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::new(0.0, 0.0, 0.0), 10.0),
            geometric_error: 10.0,
            refine: None,
            transform: None,
            content: None,
            contents: None,
            children: vec![],
            viewer_request_volume: None,
            extras: None,
        };
        // child1 位于 z=500（离相机较近）
        let child1 = Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::new(0.0, 0.0, 500.0), 10.0),
            geometric_error: 10.0,
            refine: None,
            transform: None,
            content: None,
            contents: None,
            children: vec![],
            viewer_request_volume: None,
            extras: None,
        };

        let camera = create_camera();
        // 传入 (索引, 引用) 对，排序后返回重排索引序列
        let children = vec![(0, &child0), (1, &child1)];
        let sorted = sort_children_by_distance(&children, &camera, &Ellipsoid::WGS84);

        // child0 离相机更远（相机在 z=1000 看向 z=0）
        // child1 在 z=500，离相机更近
        // 降序排序：child0 在前（更远）
        assert_eq!(sorted[0], 0);
        assert_eq!(sorted[1], 1);
    }

/// 有子瓦片且 SSE 超阈时可遍历；叶瓦片仅在有隐式内容时才可继续遍历。
#[test]
    fn test_can_traverse() {
        // 内部瓦片：有子且 SSE(20)>阈值(16) 可遍历；SSE(10)<=阈值不可
        let tile = create_tile(100.0, "test.b3dm", vec![create_leaf_tile(10.0, "c.b3dm")]);
        assert!(can_traverse(&tile, 20.0, 16.0, false));
        assert!(!can_traverse(&tile, 10.0, 16.0, false));

        // 叶瓦片：无子时默认不可遍历
        let leaf = create_leaf_tile(10.0, "leaf.b3dm");
        assert!(!can_traverse(&leaf, 100.0, 16.0, false));
        // 有隐式内容时，即使无子瓦片也可遍历
        assert!(can_traverse(&leaf, 100.0, 16.0, true));
    }

/// 默认遍历上下文：Base 策略、预加载祖先、后代限制 20。
#[test]
    fn test_traversal_context_default() {
        let ctx = TraversalContext::default();
        // 默认策略为 Base
        assert_eq!(ctx.strategy, TraversalStrategy::Base);
        // 默认预加载祖先
        assert!(ctx.preload_ancestors);
        // 默认后代加载限制 20
        assert_eq!(ctx.loading_descendant_limit, 20);
    }

/// 默认遍历结果应为空（未选中/未请求、计数归零）。
#[test]
    fn test_traversal_result_default() {
        let result = TraversalResult::default();
        // 初始无选中瓦片
        assert!(result.selected_tiles.is_empty());
        // 初始无加载请求
        assert!(result.requested_tiles.is_empty());
        // 访问计数归零
        assert_eq!(result.visited_count, 0);
        // 深度归零
        assert_eq!(result.max_depth, 0);
    }
}
