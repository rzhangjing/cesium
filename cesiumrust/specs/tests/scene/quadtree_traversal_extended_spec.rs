//! QuadtreePrimitive traversal extended specs - ported from QuadtreePrimitiveSpec.js
//!
//! Tests QuadtreeTile SSE computation, children coordinates,
//! QuadtreePrimitive.traverse with mock tile provider, TraversalResult.

use cesium_geospatial::bounding::BoundingSphere;
use cesium_quadtree::{
    QuadtreeConfig, QuadtreePrimitive, QuadtreeTile, TileState, TraversalResult,
};
use glam::DVec3;

fn make_tile(x: u32, y: u32, level: u32, geometric_error: f64) -> QuadtreeTile {
    QuadtreeTile::new(
        x,
        y,
        level,
        BoundingSphere {
            center: DVec3::new(0.0, 0.0, 0.0),
            radius: 1000000.0,
        },
        geometric_error,
    )
}

fn make_loaded_tile(x: u32, y: u32, level: u32, geometric_error: f64) -> QuadtreeTile {
    let mut tile = make_tile(x, y, level, geometric_error);
    tile.state = TileState::Loaded;
    tile
}

// ─── QuadtreeTile ──────────────────────────────────────────────────────────

#[test]
fn tile_new_defaults() {
    let tile = make_tile(0, 0, 0, 100.0);
    assert_eq!(tile.x, 0);
    assert_eq!(tile.y, 0);
    assert_eq!(tile.level, 0);
    assert!((tile.geometric_error - 100.0).abs() < 1e-10);
    assert!(tile.has_content);
    assert!(tile.refineable);
    assert_eq!(tile.state, TileState::Unloaded);
}

#[test]
fn tile_children_coords_root() {
    let tile = make_tile(0, 0, 0, 100.0);
    let children = tile.children_coords();
    assert_eq!(children[0], (0, 0));
    assert_eq!(children[1], (1, 0));
    assert_eq!(children[2], (0, 1));
    assert_eq!(children[3], (1, 1));
}

#[test]
fn tile_children_coords_level1() {
    let tile = make_tile(1, 1, 1, 50.0);
    let children = tile.children_coords();
    assert_eq!(children[0], (2, 2));
    assert_eq!(children[1], (3, 2));
    assert_eq!(children[2], (2, 3));
    assert_eq!(children[3], (3, 3));
}

#[test]
fn tile_children_coords_level2() {
    let tile = make_tile(3, 2, 2, 25.0);
    let children = tile.children_coords();
    assert_eq!(children[0], (6, 4));
    assert_eq!(children[1], (7, 4));
    assert_eq!(children[2], (6, 5));
    assert_eq!(children[3], (7, 5));
}

#[test]
fn tile_screen_space_error_close_camera() {
    let mut tile = make_tile(0, 0, 0, 1000.0);
    tile.bounding_sphere = BoundingSphere {
        center: DVec3::ZERO,
        radius: 100.0,
    };

    // Camera very close → high SSE
    let camera = DVec3::new(0.0, 0.0, 200.0);
    let sse = tile.compute_screen_space_error(camera, 1080.0, std::f64::consts::FRAC_PI_4);

    // distance = 200 - 100 = 100
    // sse_denom = 2 * tan(PI/8) ≈ 0.828
    // SSE = (1000 * 1080) / (100 * 0.828) ≈ 13037
    assert!(sse > 10000.0);
}

#[test]
fn tile_screen_space_error_far_camera() {
    let mut tile = make_tile(0, 0, 0, 1000.0);
    tile.bounding_sphere = BoundingSphere {
        center: DVec3::ZERO,
        radius: 100.0,
    };

    // Camera far away → low SSE
    let camera = DVec3::new(0.0, 0.0, 1000000.0);
    let sse = tile.compute_screen_space_error(camera, 1080.0, std::f64::consts::FRAC_PI_4);

    // distance ≈ 999900
    // SSE = (1000 * 1080) / (999900 * 0.828) ≈ 1.3
    assert!(sse < 5.0);
}

#[test]
fn tile_screen_space_error_minimum_distance() {
    let mut tile = make_tile(0, 0, 0, 100.0);
    tile.bounding_sphere = BoundingSphere {
        center: DVec3::ZERO,
        radius: 1000.0,
    };

    // Camera inside bounding sphere → distance clamped to 1.0
    let camera = DVec3::new(0.0, 0.0, 500.0);
    let sse = tile.compute_screen_space_error(camera, 1080.0, std::f64::consts::FRAC_PI_4);

    // distance = max(500 - 1000, 1.0) = 1.0
    // SSE = (100 * 1080) / (1.0 * 0.828) ≈ 130,374
    assert!(sse > 100000.0);
}

// ─── TileState ─────────────────────────────────────────────────────────────

#[test]
fn tile_state_default() {
    assert_eq!(TileState::default(), TileState::Unloaded);
}

#[test]
fn tile_state_variants() {
    let states = [
        TileState::Unloaded,
        TileState::Loading,
        TileState::Loaded,
        TileState::Rendered,
        TileState::Refined,
    ];
    // All distinct
    for i in 0..states.len() {
        for j in (i + 1)..states.len() {
            assert_ne!(states[i], states[j]);
        }
    }
}

// ─── QuadtreeConfig ────────────────────────────────────────────────────────

#[test]
fn quadtree_config_defaults() {
    let config = QuadtreeConfig::default();
    assert!((config.maximum_screen_space_error - 2.0).abs() < 1e-10);
    assert_eq!(config.maximum_level, 22);
    assert_eq!(config.minimum_level, 0);
    assert!(!config.fog_culling);
}

// ─── QuadtreePrimitive Traversal ──────────────────────────────────────────

#[test]
fn traverse_single_tile_no_refine() {
    // Camera far away → SSE below threshold → no refinement
    let root = make_tile(0, 0, 0, 100.0);
    let config = QuadtreeConfig {
        maximum_screen_space_error: 2.0,
        maximum_level: 22,
        ..Default::default()
    };
    let primitive = QuadtreePrimitive::new(vec![root], config);

    let camera = DVec3::new(0.0, 0.0, 10000000.0); // very far
    let result = primitive.traverse(camera, 1080.0, std::f64::consts::FRAC_PI_4, &|_, _, _| None);

    assert_eq!(result.tiles_visited, 1);
    assert_eq!(result.tiles_to_render.len(), 1);
    assert_eq!(result.max_depth, 0);
}

#[test]
fn traverse_refines_with_loaded_children() {
    // Camera close → SSE above threshold → refine
    let mut root = make_tile(0, 0, 0, 100000.0);
    root.bounding_sphere = BoundingSphere {
        center: DVec3::ZERO,
        radius: 100.0,
    };

    let config = QuadtreeConfig {
        maximum_screen_space_error: 2.0,
        maximum_level: 1, // limit to one level of refinement
        ..Default::default()
    };
    let primitive = QuadtreePrimitive::new(vec![root], config);

    let camera = DVec3::new(0.0, 0.0, 200.0); // close

    // Provide loaded children
    let result = primitive.traverse(camera, 1080.0, std::f64::consts::FRAC_PI_4, &|x, y, level| {
        let mut child = make_loaded_tile(x, y, level, 50000.0);
        child.bounding_sphere = BoundingSphere {
            center: DVec3::ZERO,
            radius: 50.0,
        };
        Some(child)
    });

    // Should have visited root + 4 children
    assert!(result.tiles_visited > 1);
    assert!(result.max_depth >= 1);
}

#[test]
fn traverse_unloaded_children_fallback_to_parent() {
    // Camera close → SSE above threshold → try refine
    // But children are unloaded → render parent as fallback
    let mut root = make_tile(0, 0, 0, 100000.0);
    root.bounding_sphere = BoundingSphere {
        center: DVec3::ZERO,
        radius: 100.0,
    };

    let config = QuadtreeConfig {
        maximum_screen_space_error: 2.0,
        maximum_level: 22,
        ..Default::default()
    };
    let primitive = QuadtreePrimitive::new(vec![root], config);

    let camera = DVec3::new(0.0, 0.0, 200.0);

    // Provide unloaded children
    let result = primitive.traverse(camera, 1080.0, std::f64::consts::FRAC_PI_4, &|x, y, level| {
        Some(make_tile(x, y, level, 50000.0)) // state = Unloaded
    });

    // Parent should be in tiles_to_render as fallback
    assert!(!result.tiles_to_render.is_empty());
    // Children should be in tiles_to_load
    assert!(!result.tiles_to_load.is_empty());
}

#[test]
fn traverse_respects_maximum_level() {
    // Tile at maximum level → should not refine even with high SSE
    let mut root = make_tile(0, 0, 22, 100000.0);
    root.bounding_sphere = BoundingSphere {
        center: DVec3::ZERO,
        radius: 100.0,
    };

    let config = QuadtreeConfig {
        maximum_screen_space_error: 2.0,
        maximum_level: 22,
        ..Default::default()
    };
    let primitive = QuadtreePrimitive::new(vec![root], config);

    let camera = DVec3::new(0.0, 0.0, 200.0); // close → high SSE
    let result = primitive.traverse(camera, 1080.0, std::f64::consts::FRAC_PI_4, &|_, _, _| None);

    // Should render root without refining
    assert_eq!(result.tiles_to_render.len(), 1);
    assert_eq!(result.max_depth, 22);
}

#[test]
fn traverse_non_refineable_tile() {
    let mut root = make_tile(0, 0, 0, 100000.0);
    root.bounding_sphere = BoundingSphere {
        center: DVec3::ZERO,
        radius: 100.0,
    };
    root.refineable = false;

    let config = QuadtreeConfig::default();
    let primitive = QuadtreePrimitive::new(vec![root], config);

    let camera = DVec3::new(0.0, 0.0, 200.0);
    let result = primitive.traverse(camera, 1080.0, std::f64::consts::FRAC_PI_4, &|_, _, _| None);

    // Should render without refining
    assert_eq!(result.tiles_to_render.len(), 1);
    assert_eq!(result.tiles_visited, 1);
}

#[test]
fn traversal_result_default() {
    let result = TraversalResult::default();
    assert!(result.tiles_to_render.is_empty());
    assert!(result.tiles_to_load.is_empty());
    assert_eq!(result.tiles_visited, 0);
    assert_eq!(result.max_depth, 0);
}

#[test]
fn traverse_multiple_roots() {
    // Two root tiles (like WGS84 hemispheres)
    let root1 = make_tile(0, 0, 0, 100.0);
    let root2 = make_tile(1, 0, 0, 100.0);

    let config = QuadtreeConfig::default();
    let primitive = QuadtreePrimitive::new(vec![root1, root2], config);

    let camera = DVec3::new(0.0, 0.0, 10000000.0);
    let result = primitive.traverse(camera, 1080.0, std::f64::consts::FRAC_PI_4, &|_, _, _| None);

    // Both roots visited and rendered
    assert_eq!(result.tiles_visited, 2);
    assert_eq!(result.tiles_to_render.len(), 2);
}
