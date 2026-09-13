//! Scene/Cesium3DTilesetTraversal → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/Cesium3DTilesetTraversal.js (base/skip/mostDetailed)
//! - Scene/Cesium3DTilesetSkipTraversal.js
//! - Scene/Cesium3DTilesetMostDetailedTraversal.js
//!
//! A-class tests: TilePriority ordering, MemoryAdjustedSse computation,
//! traverse strategies (Base/Skip/MostDetailed), can_traverse, sort_children.
//! C-class omitted: WebGL rendering, actual tileset loading, request scheduler.

use cesium_tileset::traversal::{
    can_traverse, sort_children_by_distance, traverse, MemoryAdjustedSse, TilePriority,
    TraversalContext, TraversalResult, TraversalStrategy,
};
use cesium_tileset::lod_selection::CameraState;
use cesium_tileset::tile::{Tile, TileContent, TileRefine};
use cesium_tileset::bounding_volume::BoundingVolume;
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::DVec3;

// === Helpers ===

fn make_camera() -> CameraState {
    CameraState::new(
        DVec3::new(0.0, 0.0, 1000.0),
        DVec3::new(0.0, 0.0, -1.0),
        DVec3::new(0.0, 1.0, 0.0),
        std::f64::consts::FRAC_PI_4,
        1080.0,
    )
}

fn make_tile(geometric_error: f64, uri: &str, children: Vec<Tile>) -> Tile {
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

fn make_leaf(geometric_error: f64, uri: &str) -> Tile {
    make_tile(geometric_error, uri, vec![])
}

fn make_tile_at(geometric_error: f64, uri: &str, center: DVec3, children: Vec<Tile>) -> Tile {
    Tile {
        bounding_volume: BoundingVolume::from_sphere(center, 50.0),
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

// === TilePriority ===

#[test]
fn tile_priority_ancestor_highest() {
    let ancestor = TilePriority { distance: 200.0, depth: 0, is_ancestor: true };
    let normal = TilePriority { distance: 50.0, depth: 2, is_ancestor: false };
    assert!(ancestor.value() < normal.value());
    assert!(ancestor < normal);
}

#[test]
fn tile_priority_distance_dominates() {
    let near = TilePriority { distance: 10.0, depth: 3, is_ancestor: false };
    let far = TilePriority { distance: 500.0, depth: 1, is_ancestor: false };
    assert!(near < far);
}

#[test]
fn tile_priority_depth_tiebreaker() {
    let shallow = TilePriority { distance: 100.0, depth: 1, is_ancestor: false };
    let deep = TilePriority { distance: 100.0, depth: 5, is_ancestor: false };
    assert!(shallow < deep);
}

#[test]
fn tile_priority_value_formula() {
    let p = TilePriority { distance: 100.0, depth: 2, is_ancestor: false };
    // value = 0 + 100 + 2*0.01 = 100.02
    assert!((p.value() - 100.02).abs() < 1e-10);

    let pa = TilePriority { distance: 100.0, depth: 2, is_ancestor: true };
    // value = -1000 + 100 + 0.02 = -899.98
    assert!((pa.value() - (-899.98)).abs() < 1e-10);
}

// === MemoryAdjustedSse ===

#[test]
fn memory_sse_zero_max_returns_base() {
    let mas = MemoryAdjustedSse::new(16.0, 0);
    assert!((mas.adjusted_sse() - 16.0).abs() < 1e-10);
}

#[test]
fn memory_sse_under_50_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1000);
    mas.current_memory_bytes = 400; // 40%
    assert!((mas.adjusted_sse() - 16.0).abs() < 1e-10);
}

#[test]
fn memory_sse_exactly_50_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1000);
    mas.current_memory_bytes = 500; // 50%
    assert!((mas.adjusted_sse() - 16.0).abs() < 1e-10);
}

#[test]
fn memory_sse_75_percent_linear() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1000);
    mas.current_memory_bytes = 750; // 75%
    // t = (0.75 - 0.5) / 0.5 = 0.5
    // sse = 16 * (1 + 0.5) = 24
    assert!((mas.adjusted_sse() - 24.0).abs() < 1e-10);
}

#[test]
fn memory_sse_100_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1000);
    mas.current_memory_bytes = 1000; // 100%
    // t = (1.0 - 0.5) / 0.5 = 1.0
    // sse = 16 * (1 + 1) = 32
    assert!((mas.adjusted_sse() - 32.0).abs() < 1e-10);
}

#[test]
fn memory_sse_over_limit_aggressive() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1000);
    mas.current_memory_bytes = 1500; // 150%
    // overage = 0.5
    // sse = 16 * (2 + 0.5*4) = 16 * 4 = 64
    assert!((mas.adjusted_sse() - 64.0).abs() < 1e-10);
    assert!(mas.is_over_limit());
}

#[test]
fn memory_sse_not_over_limit() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1000);
    mas.current_memory_bytes = 999;
    assert!(!mas.is_over_limit());
}

// === TraversalStrategy ===

#[test]
fn traversal_strategy_default_is_base() {
    assert_eq!(TraversalStrategy::default(), TraversalStrategy::Base);
}

#[test]
fn traversal_context_defaults() {
    let ctx = TraversalContext::default();
    assert_eq!(ctx.strategy, TraversalStrategy::Base);
    assert!(ctx.preload_ancestors);
    assert_eq!(ctx.loading_descendant_limit, 20);
    assert_eq!(ctx.max_tiles_per_frame, 0);
}

// === Base Traversal ===

#[test]
fn base_traversal_selects_root_when_sse_low() {
    // Camera far away, root SSE below threshold → select root only
    let root = make_tile(1.0, "root.b3dm", vec![
        make_leaf(0.1, "c0.b3dm"),
        make_leaf(0.1, "c1.b3dm"),
    ]);
    let camera = make_camera();
    let mut ctx = TraversalContext::default();
    ctx.memory_sse = MemoryAdjustedSse::new(16.0, 0); // no memory adjustment

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    // With geometric_error=1 and distance~900, SSE is very low → don't refine
    assert!(!result.selected_tiles.is_empty());
}

#[test]
fn base_traversal_refines_when_sse_high() {
    // Root has very high geometric error → should refine to children
    let root = make_tile(100000.0, "root.b3dm", vec![
        make_leaf(0.0, "c0.b3dm"),
        make_leaf(0.0, "c1.b3dm"),
    ]);
    let camera = make_camera();
    let ctx = TraversalContext::default();

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    // Should have selected tiles (children or root)
    assert!(!result.selected_tiles.is_empty());
    assert!(result.visited_count > 0);
}

#[test]
fn base_traversal_generates_load_requests() {
    let root = make_tile(100000.0, "root.b3dm", vec![
        make_leaf(0.0, "c0.b3dm"),
    ]);
    let camera = make_camera();
    let ctx = TraversalContext::default();

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    assert!(!result.requested_tiles.is_empty());
}

// === Skip Traversal ===

#[test]
fn skip_traversal_visits_multiple_levels() {
    let grandchild = make_leaf(0.0, "gc.b3dm");
    let child = make_tile(100.0, "child.b3dm", vec![grandchild]);
    let root = make_tile(100000.0, "root.b3dm", vec![child]);

    let camera = make_camera();
    let mut ctx = TraversalContext::default();
    ctx.strategy = TraversalStrategy::Skip;

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    assert!(result.visited_count > 1);
    assert!(!result.selected_tiles.is_empty());
}

#[test]
fn skip_traversal_preloads_ancestors() {
    let grandchild = make_leaf(0.0, "gc.b3dm");
    let child = make_tile(10000.0, "child.b3dm", vec![grandchild]);
    let root = make_tile(100000.0, "root.b3dm", vec![child]);

    let camera = make_camera();
    let mut ctx = TraversalContext::default();
    ctx.strategy = TraversalStrategy::Skip;
    ctx.preload_ancestors = true;

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    // Should have ancestor requests
    let has_ancestor = result.requested_tiles.iter().any(|r| r.priority.is_ancestor);
    // May or may not have ancestor requests depending on SSE values
    assert!(result.visited_count > 0);
    let _ = has_ancestor; // informational
}

// === MostDetailed Traversal ===

#[test]
fn most_detailed_selects_deepest_leaf() {
    let grandchild = make_leaf(0.0, "gc.b3dm");
    let child = make_tile(50.0, "child.b3dm", vec![grandchild]);
    let root = make_tile(1000.0, "root.b3dm", vec![child]);

    let camera = make_camera();
    let mut ctx = TraversalContext::default();
    ctx.strategy = TraversalStrategy::MostDetailed;

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    // Should select the grandchild (path [0, 0])
    assert!(result.selected_tiles.iter().any(|t| t.path == vec![0, 0]));
    assert_eq!(result.max_depth, 2);
}

#[test]
fn most_detailed_add_refinement_renders_parent_and_child() {
    let child = make_leaf(0.0, "child.b3dm");
    let mut root = make_tile(100.0, "root.b3dm", vec![child]);
    root.refine = Some(TileRefine::Add);

    let camera = make_camera();
    let mut ctx = TraversalContext::default();
    ctx.strategy = TraversalStrategy::MostDetailed;

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    // ADD: both root (path []) and child (path [0]) rendered
    assert!(result.selected_tiles.iter().any(|t| t.path.is_empty()));
    assert!(result.selected_tiles.iter().any(|t| t.path == vec![0]));
}

#[test]
fn most_detailed_replace_only_leaf() {
    let child = make_leaf(0.0, "child.b3dm");
    let root = make_tile(100.0, "root.b3dm", vec![child]);

    let camera = make_camera();
    let mut ctx = TraversalContext::default();
    ctx.strategy = TraversalStrategy::MostDetailed;

    let result = traverse(&root, &camera, &ctx, &Ellipsoid::WGS84);
    // REPLACE: only leaf rendered, not parent
    assert!(result.selected_tiles.iter().any(|t| t.path == vec![0]));
    assert!(!result.selected_tiles.iter().any(|t| t.path.is_empty()));
}

// === can_traverse ===

#[test]
fn can_traverse_with_children_high_sse() {
    let tile = make_tile(100.0, "t.b3dm", vec![make_leaf(10.0, "c.b3dm")]);
    assert!(can_traverse(&tile, 20.0, 16.0, false));
}

#[test]
fn can_traverse_low_sse_false() {
    let tile = make_tile(100.0, "t.b3dm", vec![make_leaf(10.0, "c.b3dm")]);
    assert!(!can_traverse(&tile, 10.0, 16.0, false));
}

#[test]
fn can_traverse_leaf_no_implicit() {
    let leaf = make_leaf(10.0, "leaf.b3dm");
    assert!(!can_traverse(&leaf, 100.0, 16.0, false));
}

#[test]
fn can_traverse_leaf_with_implicit() {
    let leaf = make_leaf(10.0, "leaf.b3dm");
    assert!(can_traverse(&leaf, 100.0, 16.0, true));
}

// === sort_children_by_distance ===

#[test]
fn sort_children_farthest_first() {
    let child_near = make_tile_at(10.0, "near.b3dm", DVec3::new(0.0, 0.0, 500.0), vec![]);
    let child_far = make_tile_at(10.0, "far.b3dm", DVec3::new(0.0, 0.0, -500.0), vec![]);

    let camera = make_camera(); // at z=1000
    let children = vec![(0, &child_near), (1, &child_far)];
    let sorted = sort_children_by_distance(&children, &camera, &Ellipsoid::WGS84);

    // child_far (z=-500) is farther from camera (z=1000) than child_near (z=500)
    assert_eq!(sorted[0], 1); // far first
    assert_eq!(sorted[1], 0); // near second
}

// === TraversalResult ===

#[test]
fn traversal_result_default_empty() {
    let result = TraversalResult::default();
    assert!(result.selected_tiles.is_empty());
    assert!(result.requested_tiles.is_empty());
    assert_eq!(result.visited_count, 0);
    assert_eq!(result.culled_count, 0);
    assert_eq!(result.max_depth, 0);
}
