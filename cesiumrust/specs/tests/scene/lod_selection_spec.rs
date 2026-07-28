//! Scene/Cesium3DTilesetTraversal + LOD selection → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Cesium3DTileset._computeScreenSpaceError
//! - Cesium3DTilesetTraversal traversal logic
//!
//! A-class tests: SSE formula, tile selection (REPLACE/ADD), distance computation.
//! C-class omitted: WebGL rendering, actual tileset loading, frame state.

use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_tileset::lod_selection::{
    compute_distance_to_tile, get_tile_by_path, select_tiles, should_refine_tile, CameraState,
    LodSelectionContext, TileSelectionResult,
};
use cesium_tileset::{BoundingVolume, Tile, TileContent, TileRefine};
use glam::DVec3;
use std::f64::consts::FRAC_PI_4;

fn test_camera() -> CameraState {
    CameraState::new(
        DVec3::new(0.0, 0.0, 1000.0),
        DVec3::new(0.0, 0.0, -1.0),
        DVec3::new(0.0, 1.0, 0.0),
        FRAC_PI_4, // 45 degrees fov
        1080.0,    // viewport height
    )
}

fn make_tile(geometric_error: f64, radius: f64, children: Vec<Tile>) -> Tile {
    Tile {
        bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, radius),
        geometric_error,
        refine: Some(TileRefine::Replace),
        transform: None,
        content: Some(TileContent {
            uri: "tile.b3dm".to_string(),
            bounding_volume: None,
            group: None,
        }),
        contents: None,
        children,
        viewer_request_volume: None,
        extras: None,
    }
}

fn make_child(error: f64) -> Tile {
    Tile {
        bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 50.0),
        geometric_error: error,
        refine: None,
        transform: None,
        content: Some(TileContent {
            uri: "child.b3dm".to_string(),
            bounding_volume: None,
            group: None,
        }),
        contents: None,
        children: vec![],
        viewer_request_volume: None,
        extras: None,
    }
}

// === SSE Formula ===

#[test]
fn sse_formula_matches_cesiumjs() {
    // CesiumJS: SSE = (geometricError * viewportHeight) / (distance * 2 * tan(fovY / 2))
    let camera = test_camera();
    let sse = camera.compute_screen_space_error(100.0, 1000.0);
    let expected = (100.0 * 1080.0) / (1000.0 * 2.0 * (FRAC_PI_4 / 2.0).tan());
    assert!((sse - expected).abs() < 1e-10);
}

#[test]
fn sse_zero_distance_returns_max() {
    let camera = test_camera();
    let sse = camera.compute_screen_space_error(100.0, 0.0);
    assert_eq!(sse, f64::MAX);
}

#[test]
fn sse_negative_distance_returns_max() {
    let camera = test_camera();
    let sse = camera.compute_screen_space_error(100.0, -10.0);
    assert_eq!(sse, f64::MAX);
}

#[test]
fn sse_increases_with_geometric_error() {
    let camera = test_camera();
    let sse_small = camera.compute_screen_space_error(10.0, 1000.0);
    let sse_large = camera.compute_screen_space_error(100.0, 1000.0);
    assert!(sse_large > sse_small);
    assert!((sse_large / sse_small - 10.0).abs() < 1e-10);
}

#[test]
fn sse_increases_with_proximity() {
    let camera = test_camera();
    let sse_far = camera.compute_screen_space_error(100.0, 10000.0);
    let sse_near = camera.compute_screen_space_error(100.0, 1000.0);
    assert!(sse_near > sse_far);
    assert!((sse_near / sse_far - 10.0).abs() < 1e-10);
}

#[test]
fn sse_scales_with_viewport_height() {
    let cam1 = CameraState::new(
        DVec3::new(0.0, 0.0, 1000.0),
        DVec3::new(0.0, 0.0, -1.0),
        DVec3::new(0.0, 1.0, 0.0),
        FRAC_PI_4,
        540.0,
    );
    let cam2 = CameraState::new(
        DVec3::new(0.0, 0.0, 1000.0),
        DVec3::new(0.0, 0.0, -1.0),
        DVec3::new(0.0, 1.0, 0.0),
        FRAC_PI_4,
        1080.0,
    );
    let sse1 = cam1.compute_screen_space_error(100.0, 1000.0);
    let sse2 = cam2.compute_screen_space_error(100.0, 1000.0);
    assert!((sse2 / sse1 - 2.0).abs() < 1e-10);
}

// === should_refine_tile ===

#[test]
fn should_refine_when_sse_exceeds_and_has_children() {
    assert!(should_refine_tile(20.0, 16.0, true));
}

#[test]
fn should_not_refine_when_sse_below_threshold() {
    assert!(!should_refine_tile(10.0, 16.0, true));
}

#[test]
fn should_not_refine_without_children() {
    assert!(!should_refine_tile(100.0, 16.0, false));
}

// === Distance computation ===

#[test]
fn distance_to_sphere_tile() {
    let camera = test_camera();
    let tile = make_tile(10.0, 100.0, vec![]);
    let distance = compute_distance_to_tile(&camera, &tile, &Ellipsoid::WGS84);
    // Camera at (0,0,1000), sphere at origin radius 100
    // Distance = |camera_pos| - radius = 1000 - 100 = 900
    assert!((distance - 900.0).abs() < 1e-10);
}

// === Tile Selection ===

#[test]
fn select_tiles_renders_leaf_when_sse_low() {
    let root = make_tile(10.0, 100.0, vec![]);
    let camera = test_camera();
    let context = LodSelectionContext::default();

    let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].result, TileSelectionResult::Render);
    assert!(selected[0].path.is_empty());
}

#[test]
fn select_tiles_refines_when_sse_high() {
    let root = make_tile(1000.0, 100.0, vec![make_child(5.0)]);
    let camera = test_camera();
    let context = LodSelectionContext::default();

    let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);
    // Should refine to child
    assert!(selected.iter().any(|t| t.path == vec![0]));
}

#[test]
fn select_tiles_add_refinement_renders_parent_and_child() {
    let mut root = make_tile(1000.0, 100.0, vec![make_child(5.0)]);
    root.refine = Some(TileRefine::Add);

    let camera = test_camera();
    let context = LodSelectionContext::default();

    let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);
    // ADD mode: both parent and child rendered
    assert!(selected.iter().any(|t| t.path.is_empty())); // parent
    assert!(selected.iter().any(|t| t.path == vec![0])); // child
}

#[test]
fn select_tiles_empty_root_refines_anyway() {
    let mut root = make_tile(10.0, 100.0, vec![make_child(5.0)]);
    root.content = None; // no content

    let camera = test_camera();
    let context = LodSelectionContext::default();

    let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);
    // Empty tile with children: refine regardless of SSE
    assert!(selected.iter().any(|t| t.path == vec![0]));
}

// === get_tile_by_path ===

#[test]
fn get_tile_by_path_root() {
    let root = make_tile(100.0, 100.0, vec![make_child(50.0)]);
    let tile = get_tile_by_path(&root, &[]);
    assert!(tile.is_some());
    assert_eq!(tile.unwrap().geometric_error, 100.0);
}

#[test]
fn get_tile_by_path_child() {
    let root = make_tile(100.0, 100.0, vec![make_child(50.0)]);
    let tile = get_tile_by_path(&root, &[0]);
    assert!(tile.is_some());
    assert_eq!(tile.unwrap().geometric_error, 50.0);
}

#[test]
fn get_tile_by_path_invalid() {
    let root = make_tile(100.0, 100.0, vec![make_child(50.0)]);
    assert!(get_tile_by_path(&root, &[1]).is_none());
    assert!(get_tile_by_path(&root, &[0, 0]).is_none());
}

// === LodSelectionContext defaults ===

#[test]
fn lod_context_default_values() {
    let ctx = LodSelectionContext::default();
    assert_eq!(ctx.maximum_screen_space_error, 16.0);
    assert!(ctx.cull_with_frustum);
    assert!(!ctx.skip_level_of_detail);
}
