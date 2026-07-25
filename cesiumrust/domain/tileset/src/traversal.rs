//! Advanced 3D Tiles traversal strategies.
//!
//! Maps to CesiumJS:
//! - `Scene/Cesium3DTilesetTraversal.js`
//! - `Scene/Cesium3DTilesetSkipTraversal.js`
//! - `Scene/Cesium3DTilesetMostDetailedTraversal.js`
//! - `Scene/Cesium3DTilesetBaseTraversal.js`

use crate::lod_selection::{CameraState, LodSelectionContext, SelectedTile, TileSelectionResult};
use crate::tile::{Tile, TileRefine};
use cesium_geospatial::ellipsoid::Ellipsoid;

/// Traversal strategy selection.
///
/// Maps to CesiumJS traversal types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraversalStrategy {
    /// Base traversal: simple top-down SSE-based refinement.
    #[default]
    Base,
    /// Skip traversal: allows skipping levels, renders parent+children simultaneously.
    Skip,
    /// Most detailed traversal: always refines to deepest available content.
    MostDetailed,
}

/// Priority for tile loading requests.
///
/// Maps to CesiumJS tile priority computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TilePriority {
    /// Distance from camera (lower = higher priority).
    pub distance: f64,
    /// Depth in the tree (lower = higher priority for ancestors).
    pub depth: u32,
    /// Whether this is an ancestor of a selected tile.
    pub is_ancestor: bool,
}

impl TilePriority {
    /// Computes a numeric priority value (lower = load first).
    pub fn value(&self) -> f64 {
        // Ancestors get highest priority (load parent before children)
        let ancestor_bonus = if self.is_ancestor { -1000.0 } else { 0.0 };
        ancestor_bonus + self.distance + (self.depth as f64) * 0.01
    }
}

impl PartialOrd for TilePriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TilePriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value()
            .partial_cmp(&other.value())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Eq for TilePriority {}

/// A tile request with priority.
#[derive(Debug, Clone)]
pub struct TileRequest {
    /// Path to the tile in the tree.
    pub path: Vec<usize>,
    /// Priority for loading.
    pub priority: TilePriority,
}

/// Memory-adjusted screen space error computation.
///
/// Maps to CesiumJS `Cesium3DTileset.memoryAdjustedScreenSpaceError`
#[derive(Debug, Clone)]
pub struct MemoryAdjustedSse {
    /// Base maximum screen space error.
    pub base_sse: f64,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Current memory usage in bytes.
    pub current_memory_bytes: u64,
}

impl MemoryAdjustedSse {
    /// Creates a new memory-adjusted SSE calculator.
    pub fn new(base_sse: f64, max_memory_bytes: u64) -> Self {
        Self {
            base_sse,
            max_memory_bytes,
            current_memory_bytes: 0,
        }
    }

    /// Computes the memory-adjusted SSE threshold.
    ///
    /// When memory usage exceeds the limit, the SSE threshold is increased
    /// to reduce detail and free memory.
    pub fn adjusted_sse(&self) -> f64 {
        if self.max_memory_bytes == 0 {
            return self.base_sse;
        }

        let usage_ratio =
            self.current_memory_bytes as f64 / self.max_memory_bytes as f64;

        if usage_ratio <= 0.5 {
            // Under 50% memory: use base SSE
            self.base_sse
        } else if usage_ratio < 1.0 {
            // 50-100%: linearly increase SSE
            let t = (usage_ratio - 0.5) / 0.5;
            self.base_sse * (1.0 + t)
        } else {
            // Over 100%: aggressively increase SSE
            let overage = usage_ratio - 1.0;
            self.base_sse * (2.0 + overage * 4.0)
        }
    }

    /// Returns true if memory is over the limit.
    pub fn is_over_limit(&self) -> bool {
        self.current_memory_bytes > self.max_memory_bytes
    }
}

/// Traversal context with all configuration.
#[derive(Debug, Clone)]
pub struct TraversalContext {
    /// LOD selection context.
    pub lod_context: LodSelectionContext,
    /// Traversal strategy.
    pub strategy: TraversalStrategy,
    /// Memory-adjusted SSE.
    pub memory_sse: MemoryAdjustedSse,
    /// Maximum number of tiles to visit per frame (0 = unlimited).
    pub max_tiles_per_frame: usize,
    /// Whether to preload ancestors.
    pub preload_ancestors: bool,
    /// Loading descendant limit.
    pub loading_descendant_limit: u32,
}

impl Default for TraversalContext {
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

/// Result of a traversal operation.
#[derive(Debug, Clone, Default)]
pub struct TraversalResult {
    /// Tiles selected for rendering.
    pub selected_tiles: Vec<SelectedTile>,
    /// Tiles requested for loading (with priority).
    pub requested_tiles: Vec<TileRequest>,
    /// Number of tiles visited.
    pub visited_count: usize,
    /// Number of tiles culled.
    pub culled_count: usize,
    /// Maximum depth reached.
    pub max_depth: u32,
}

/// Performs tile traversal using the configured strategy.
pub fn traverse(
    root: &Tile,
    camera: &CameraState,
    context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    match context.strategy {
        TraversalStrategy::Base => traverse_base(root, camera, context, ellipsoid),
        TraversalStrategy::Skip => traverse_skip(root, camera, context, ellipsoid),
        TraversalStrategy::MostDetailed => {
            traverse_most_detailed(root, camera, context, ellipsoid)
        }
    }
}

/// Base traversal: simple top-down SSE-based refinement.
fn traverse_base(
    root: &Tile,
    camera: &CameraState,
    context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    let mut result = TraversalResult::default();
    let effective_sse = context.memory_sse.adjusted_sse();

    let mut ctx = context.lod_context.clone();
    ctx.maximum_screen_space_error = effective_sse;

    result.selected_tiles =
        crate::lod_selection::select_tiles(root, camera, &ctx, ellipsoid);
    result.visited_count = result.selected_tiles.len();

    // Generate load requests for selected tiles
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

/// Skip traversal: allows skipping levels of the tree.
///
/// Maps to CesiumJS `Cesium3DTilesetSkipTraversal.selectTiles`
///
/// Key differences from base traversal:
/// - Can render parent and child tiles simultaneously
/// - Skips intermediate levels when children are not yet loaded
/// - Uses descendant selection depth of 2
fn traverse_skip(
    root: &Tile,
    camera: &CameraState,
    context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    let mut result = TraversalResult::default();
    let effective_sse = context.memory_sse.adjusted_sse();

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

    // Sort requests by priority
    result.requested_tiles.sort_by_key(|a| a.priority);

    result
}

/// Recursive helper for skip traversal.
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

    let distance = tile.bounding_volume.distance_to(camera.position, ellipsoid);
    let sse = camera.compute_screen_space_error(tile.geometric_error, distance);
    let refine_mode = tile.effective_refine(parent_refine);
    let has_children = !tile.children.is_empty();

    // Check if we should refine
    let should_refine = has_children && sse > max_sse;

    if !should_refine {
        // Render this tile
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
            // Empty tile: must refine
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

    // Should refine: check if children are ready
    // In skip traversal, we render the parent if children aren't ready
    // and also try to load children (skip levels if needed)

    // For ADD refinement, always render parent
    if refine_mode == TileRefine::Add && tile.has_content() {
        result.selected_tiles.push(SelectedTile {
            path: path.to_vec(),
            result: TileSelectionResult::Render,
            screen_space_error: sse,
            distance_to_camera: distance,
        });
    }

    // Traverse children with skip logic
    // Skip traversal: look ahead 2 levels (descendantSelectionDepth = 2)
    let mut any_child_rendered = false;
    for (i, child) in tile.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(i);

        let child_distance =
            child.bounding_volume.distance_to(camera.position, ellipsoid);
        let child_sse =
            camera.compute_screen_space_error(child.geometric_error, child_distance);

        // If child SSE is still too high and has grandchildren, skip to grandchildren
        if !child.children.is_empty() && child_sse > max_sse {
            // Skip level: render child as ancestor, traverse grandchildren
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
            // Normal traversal for this child
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

    // If no children were rendered and this tile has content, render it as fallback
    if !any_child_rendered && tile.has_content() && refine_mode == TileRefine::Replace {
        result.selected_tiles.push(SelectedTile {
            path: path.to_vec(),
            result: TileSelectionResult::Render,
            screen_space_error: sse,
            distance_to_camera: distance,
        });
    }
}

/// Most detailed traversal: always refines to the deepest available content.
///
/// Maps to CesiumJS `Cesium3DTilesetMostDetailedTraversal.selectTiles`
///
/// This traversal is used for picking and other operations where
/// the most detailed tile is needed regardless of SSE.
fn traverse_most_detailed(
    root: &Tile,
    camera: &CameraState,
    _context: &TraversalContext,
    ellipsoid: &Ellipsoid,
) -> TraversalResult {
    let mut result = TraversalResult::default();

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

/// Recursive helper for most detailed traversal.
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

    // Always try to refine to children (most detailed)
    if has_children {
        // For ADD refinement, also render parent
        if refine_mode == TileRefine::Add && tile.has_content() {
            result.selected_tiles.push(SelectedTile {
                path: path.to_vec(),
                result: TileSelectionResult::Render,
                screen_space_error: sse,
                distance_to_camera: distance,
            });
        }

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
        // Leaf tile with content: render it
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
    }
}

/// Sorts children by distance to camera (farthest first for stack-based traversal).
///
/// Maps to CesiumJS `Cesium3DTilesetTraversal.sortChildrenByDistanceToCamera`
pub fn sort_children_by_distance(
    children: &[(usize, &Tile)],
    camera: &CameraState,
    ellipsoid: &Ellipsoid,
) -> Vec<usize> {
    let mut indexed: Vec<(usize, f64)> = children
        .iter()
        .map(|(i, tile)| {
            let dist = tile.bounding_volume.distance_to(camera.position, ellipsoid);
            (*i, dist)
        })
        .collect();

    // Sort by distance descending (farthest first)
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.into_iter().map(|(i, _)| i).collect()
}

/// Checks if a tile can be traversed (has children and SSE exceeds threshold).
///
/// Maps to CesiumJS `Cesium3DTilesetTraversal.canTraverse`
pub fn can_traverse(
    tile: &Tile,
    sse: f64,
    max_sse: f64,
    has_implicit_content: bool,
) -> bool {
    if tile.children.is_empty() && !has_implicit_content {
        return false;
    }
    sse > max_sse
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounding_volume::BoundingVolume;
    use crate::tile::TileContent;
    use glam::DVec3;

    fn create_camera() -> CameraState {
        CameraState::new(
            DVec3::new(0.0, 0.0, 1000.0),
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            std::f64::consts::FRAC_PI_4,
            1080.0,
        )
    }

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

    fn create_leaf_tile(geometric_error: f64, uri: &str) -> Tile {
        create_tile(geometric_error, uri, vec![])
    }

    #[test]
    fn test_traversal_strategy_default() {
        assert_eq!(TraversalStrategy::default(), TraversalStrategy::Base);
    }

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
        // Ancestor should have higher priority (lower value)
        assert!(p1.value() < p2.value());
    }

    #[test]
    fn test_memory_adjusted_sse_under_limit() {
        let mas = MemoryAdjustedSse::new(16.0, 1000);
        assert_eq!(mas.adjusted_sse(), 16.0);
    }

    #[test]
    fn test_memory_adjusted_sse_half_usage() {
        let mut mas = MemoryAdjustedSse::new(16.0, 1000);
        mas.current_memory_bytes = 500; // 50%
        assert_eq!(mas.adjusted_sse(), 16.0);
    }

    #[test]
    fn test_memory_adjusted_sse_high_usage() {
        let mut mas = MemoryAdjustedSse::new(16.0, 1000);
        mas.current_memory_bytes = 750; // 75%
        let sse = mas.adjusted_sse();
        assert!(sse > 16.0);
        assert!(sse < 32.0);
    }

    #[test]
    fn test_memory_adjusted_sse_over_limit() {
        let mut mas = MemoryAdjustedSse::new(16.0, 1000);
        mas.current_memory_bytes = 1500; // 150%
        let sse = mas.adjusted_sse();
        assert!(sse > 32.0);
        assert!(mas.is_over_limit());
    }

    #[test]
    fn test_base_traversal() {
        let root = create_tile(
            1000.0,
            "root.b3dm",
            vec![
                create_leaf_tile(10.0, "child0.b3dm"),
                create_leaf_tile(10.0, "child1.b3dm"),
            ],
        );
        let camera = create_camera();
        let context = TraversalContext::default();

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);
        assert!(!result.selected_tiles.is_empty());
    }

    #[test]
    fn test_skip_traversal() {
        // Create a 3-level tree
        let grandchild = create_leaf_tile(1.0, "gc.b3dm");
        let child = create_tile(100.0, "child.b3dm", vec![grandchild]);
        let root = create_tile(1000.0, "root.b3dm", vec![child]);

        let camera = create_camera();
        let mut context = TraversalContext::default();
        context.strategy = TraversalStrategy::Skip;

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);
        assert!(!result.selected_tiles.is_empty());
        // Skip traversal should have visited multiple levels
        assert!(result.visited_count > 0);
    }

    #[test]
    fn test_most_detailed_traversal() {
        // Create a 3-level tree
        let grandchild = create_leaf_tile(0.0, "gc.b3dm");
        let child = create_tile(50.0, "child.b3dm", vec![grandchild]);
        let root = create_tile(1000.0, "root.b3dm", vec![child]);

        let camera = create_camera();
        let mut context = TraversalContext::default();
        context.strategy = TraversalStrategy::MostDetailed;

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);

        // Most detailed should select the deepest tile (grandchild)
        assert!(result.selected_tiles.iter().any(|t| t.path == vec![0, 0]));
        assert_eq!(result.max_depth, 2);
    }

    #[test]
    fn test_most_detailed_add_refinement() {
        let child = create_leaf_tile(0.0, "child.b3dm");
        let mut root = create_tile(100.0, "root.b3dm", vec![child]);
        root.refine = Some(TileRefine::Add);

        let camera = create_camera();
        let mut context = TraversalContext::default();
        context.strategy = TraversalStrategy::MostDetailed;

        let result = traverse(&root, &camera, &context, &Ellipsoid::WGS84);

        // ADD refinement: both parent and child should be rendered
        assert!(result.selected_tiles.iter().any(|t| t.path.is_empty()));
        assert!(result.selected_tiles.iter().any(|t| t.path == vec![0]));
    }

    #[test]
    fn test_sort_children_by_distance() {
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
        let children = vec![(0, &child0), (1, &child1)];
        let sorted = sort_children_by_distance(&children, &camera, &Ellipsoid::WGS84);

        // child0 is farther from camera (at z=1000 looking at z=0)
        // child1 is at z=500, closer to camera
        // Sorted descending: child0 first (farther)
        assert_eq!(sorted[0], 0);
        assert_eq!(sorted[1], 1);
    }

    #[test]
    fn test_can_traverse() {
        let tile = create_tile(100.0, "test.b3dm", vec![create_leaf_tile(10.0, "c.b3dm")]);
        assert!(can_traverse(&tile, 20.0, 16.0, false));
        assert!(!can_traverse(&tile, 10.0, 16.0, false));

        let leaf = create_leaf_tile(10.0, "leaf.b3dm");
        assert!(!can_traverse(&leaf, 100.0, 16.0, false));
        // With implicit content, can traverse even without children
        assert!(can_traverse(&leaf, 100.0, 16.0, true));
    }

    #[test]
    fn test_traversal_context_default() {
        let ctx = TraversalContext::default();
        assert_eq!(ctx.strategy, TraversalStrategy::Base);
        assert!(ctx.preload_ancestors);
        assert_eq!(ctx.loading_descendant_limit, 20);
    }

    #[test]
    fn test_traversal_result_default() {
        let result = TraversalResult::default();
        assert!(result.selected_tiles.is_empty());
        assert!(result.requested_tiles.is_empty());
        assert_eq!(result.visited_count, 0);
        assert_eq!(result.max_depth, 0);
    }
}
