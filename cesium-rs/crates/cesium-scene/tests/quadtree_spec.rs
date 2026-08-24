//! B4-2 spec mirror: pure-logic cases from
//! `packages/engine/Specs/Scene/QuadtreePrimitiveSpec.js` and
//! `packages/engine/Specs/Scene/QuadtreeTileSpec.js`.
//!
//! Standalone integration-test entry — the specs aggregator under
//! `specs/tests/` is intentionally untouched.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::math::CesiumMath;
use cesium_core::web_mercator_tiling_scheme::WebMercatorTilingScheme;
use cesium_scene::frame_state::FrameState;
use cesium_scene::quadtree_primitive::QuadtreePrimitive;
use cesium_scene::quadtree_tile::{
    QuadtreeTile, CHILD_NORTHEAST, CHILD_NORTHWEST, CHILD_SOUTHEAST, CHILD_SOUTHWEST,
};
use cesium_test_utils::assert_approx_eq_f64;

/// Default drawing-buffer height used by the traversal fixtures.
const BUFFER_HEIGHT: f64 = 600.0;
/// Default vertical FOV (60°) — mirrors the FrameState default sse denominator.
const FOV: f64 = std::f64::consts::FRAC_PI_3;

fn frame_state(camera_position: Cartesian3) -> FrameState {
    let mut frame_state = FrameState::new();
    frame_state.camera_position = camera_position;
    frame_state.drawing_buffer_width = 800;
    frame_state.drawing_buffer_height = BUFFER_HEIGHT as u32;
    frame_state.frame_number = 1;
    frame_state
}

/// The level-zero error threshold distance: tiles closer than
/// `d0 = geometricError * height / (maxSse * sseDenominator)` subdivide.
fn level_zero_threshold(qt: &QuadtreePrimitive) -> f64 {
    let sse_denominator = 2.0 * (FOV * 0.5).tan();
    qt.level_zero_maximum_geometric_error() * BUFFER_HEIGHT
        / (qt.maximum_screen_space_error() * sse_denominator)
}

/// Places the camera on the +Z axis so its distance to the west root tile's
/// bounding sphere is exactly `surface_distance`. Both geographic roots are
/// symmetric about that axis, so the east tile sees the same distance.
fn camera_for_surface_distance(qt: &QuadtreePrimitive, surface_distance: f64) -> Cartesian3 {
    let sphere = &qt.root_tiles()[0].bounding_sphere;
    let lateral = sphere.center.x * sphere.center.x + sphere.center.y * sphere.center.y;
    let to_center = sphere.radius + surface_distance;
    let z = (to_center * to_center - lateral).sqrt();
    Cartesian3::new(0.0, 0.0, z)
}

// ---- Root tile initialization ----

/// QuadtreePrimitiveSpec: the default GeographicTilingScheme yields two
/// level-zero tiles covering the full longitude range.
#[test]
fn geographic_roots_cover_the_globe() {
    let quadtree = QuadtreePrimitive::new();
    let roots = quadtree.root_tiles();
    assert_eq!(roots.len(), 2);

    let half_pi = std::f64::consts::FRAC_PI_2;
    let pi = std::f64::consts::PI;
    for (i, tile) in roots.iter().enumerate() {
        assert_eq!(tile.level, 0);
        assert_eq!(tile.y, 0);
        assert_eq!(tile.x, i as i32);
        assert_approx_eq_f64!(tile.rectangle.south, -half_pi);
        assert_approx_eq_f64!(tile.rectangle.north, half_pi);
        assert_approx_eq_f64!(
            tile.geometric_error,
            quadtree.level_zero_maximum_geometric_error()
        );
    }
    assert_approx_eq_f64!(roots[0].rectangle.west, -pi);
    assert_approx_eq_f64!(roots[0].rectangle.east, 0.0);
    assert_approx_eq_f64!(roots[1].rectangle.west, 0.0);
    assert_approx_eq_f64!(roots[1].rectangle.east, pi);
}

/// The default WebMercatorTilingScheme starts from a single root tile.
#[test]
fn web_mercator_has_a_single_root() {
    let quadtree = QuadtreePrimitive::with_tiling_scheme(
        Box::new(WebMercatorTilingScheme::new(None, None, None, None, None)),
        None,
    );
    assert_eq!(quadtree.root_tiles().len(), 1);
    assert_eq!(quadtree.root_tiles()[0].x, 0);
    assert_eq!(quadtree.root_tiles()[0].level, 0);
}

// ---- Geometric error ----

/// Level-zero error mirrors CesiumJS
/// `TerrainProvider.getEstimatedLevelZeroGeometricErrorForAHeightmap`:
/// `maxRadius * 2π * 0.25 / (tileImageWidth * numberOfTilesAtLevelZero)`.
#[test]
fn level_zero_geometric_error_matches_the_heightmap_estimate() {
    let quadtree = QuadtreePrimitive::new();
    let expected = Ellipsoid::WGS84.maximum_radius() * 2.0 * std::f64::consts::PI * 0.25
        / (65.0 * 2.0);
    assert_approx_eq_f64!(quadtree.level_zero_maximum_geometric_error(), expected);
}

/// `getLevelMaximumGeometricError` halves per level (`error >> level`).
#[test]
fn level_maximum_geometric_error_halves_per_level() {
    let quadtree = QuadtreePrimitive::new();
    let level_zero = quadtree.level_zero_maximum_geometric_error();
    assert_approx_eq_f64!(quadtree.get_level_maximum_geometric_error(0), level_zero);
    assert_approx_eq_f64!(quadtree.get_level_maximum_geometric_error(1), level_zero / 2.0);
    assert_approx_eq_f64!(quadtree.get_level_maximum_geometric_error(3), level_zero / 8.0);
}

// ---- QuadtreeTile child layout ----

/// QuadtreeTileSpec: children are `(2x,2y)` NW / `(2x+1,2y)` NE /
/// `(2x,2y+1)` SW / `(2x+1,2y+1)` SE at `level+1`, each with half the
/// geometric error and the matching rectangle quadrant.
#[test]
fn ensure_children_lays_out_quadrants() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut tile = QuadtreeTile::from_tiling_scheme(&scheme, 1, 1, 1, 100.0);
    assert!(tile.children.is_empty());
    tile.ensure_children(&scheme);
    assert_eq!(tile.children.len(), 4);

    let expected = [
        (CHILD_NORTHWEST, 2, 2),
        (CHILD_NORTHEAST, 3, 2),
        (CHILD_SOUTHWEST, 2, 3),
        (CHILD_SOUTHEAST, 3, 3),
    ];
    for (slot, x, y) in expected {
        let child = &tile.children[slot];
        assert_eq!(child.level, 2);
        assert_eq!(child.x, x);
        assert_eq!(child.y, y);
        assert_approx_eq_f64!(child.geometric_error, 50.0);
    }

    // The NW child occupies the parent's north-west quadrant.
    let center_lon = (tile.rectangle.west + tile.rectangle.east) * 0.5;
    let center_lat = (tile.rectangle.south + tile.rectangle.north) * 0.5;
    let nw = &tile.children[CHILD_NORTHWEST].rectangle;
    assert_approx_eq_f64!(nw.west, tile.rectangle.west);
    assert_approx_eq_f64!(nw.east, center_lon);
    assert_approx_eq_f64!(nw.south, center_lat);
    assert_approx_eq_f64!(nw.north, tile.rectangle.north);

    // Children are created only once.
    tile.ensure_children(&scheme);
    assert_eq!(tile.children.len(), 4);
}

// ---- SSE traversal ----

/// A camera beyond the SSE threshold renders only the two roots
/// (no-refinement boundary) and the stored SSE matches the CesiumJS
/// formula `error = geometricError * height / (distance * sseDenominator)`.
#[test]
fn far_camera_renders_roots_without_refining() {
    let mut quadtree = QuadtreePrimitive::new();
    let threshold = level_zero_threshold(&quadtree);
    let camera = camera_for_surface_distance(&quadtree, threshold * 1.5);

    quadtree.update(&frame_state(camera));

    assert_eq!(quadtree.tiles_to_render().len(), 2);
    assert_eq!(quadtree.debug_tiles_visited, 2);
    assert_eq!(quadtree.debug_max_depth_visited, 0);

    let sse_denominator = 2.0 * (FOV * 0.5).tan();
    for tile in quadtree.tiles_to_render() {
        assert_eq!(tile.level, 0);
        assert!(tile.screen_space_error < quadtree.maximum_screen_space_error());
        assert!(tile.camera_distance > threshold);
        let expected_sse = tile.geometric_error * BUFFER_HEIGHT
            / (tile.camera_distance * sse_denominator);
        assert_approx_eq_f64!(tile.screen_space_error, expected_sse, CesiumMath::EPSILON6);
    }

    // Synchronous semantics: nothing left in the load queues.
    assert!(quadtree.tiles_loaded());

    // A second update rebuilds the render list from scratch.
    quadtree.update(&frame_state(camera));
    assert_eq!(quadtree.tiles_to_render().len(), 2);
}

/// A camera inside the refinement threshold subdivides both roots; with
/// `maximumLevel = 1` the children cannot refine further (canRefine is
/// false) and all eight level-1 tiles render.
#[test]
fn near_camera_refines_to_the_maximum_level() {
    let mut quadtree = QuadtreePrimitive::new();
    quadtree.set_maximum_level(Some(1));
    let threshold = level_zero_threshold(&quadtree);
    let camera = camera_for_surface_distance(&quadtree, threshold * 0.5);

    quadtree.update(&frame_state(camera));

    assert_eq!(quadtree.tiles_to_render().len(), 8);
    assert_eq!(quadtree.debug_tiles_visited, 10);
    assert_eq!(quadtree.debug_max_depth_visited, 1);
    for tile in quadtree.tiles_to_render() {
        assert_eq!(tile.level, 1);
    }

    // The refined roots now own four children each.
    for root in quadtree.root_tiles() {
        assert_eq!(root.children.len(), 4);
    }
}

/// SSE distance-floor discipline (cesiumrust historical lesson): when the
/// camera is inside a tile's bounding sphere the distance clamps to exactly
/// zero — never to a floor above the camera's actual minimum distance. The
/// zero distance yields an infinite SSE, forcing refinement all the way to
/// the maximum level instead of a blurry under-subdivided screen.
#[test]
fn camera_inside_bounding_sphere_clamps_distance_to_zero() {
    let mut quadtree = QuadtreePrimitive::new();
    quadtree.set_maximum_level(Some(2));

    // The origin lies inside both geographic root bounding spheres.
    quadtree.update(&frame_state(Cartesian3::ZERO));

    assert_eq!(quadtree.tiles_to_render().len(), 32); // 2 roots * 4^2
    assert_eq!(quadtree.debug_max_depth_visited, 2);
    for tile in quadtree.tiles_to_render() {
        assert_eq!(tile.level, 2);
    }

    // The roots themselves recorded the clamped distance: exactly zero, and
    // an infinite screen-space error that drove the refinement.
    for root in quadtree.root_tiles() {
        assert_eq!(root.camera_distance, 0.0);
        assert_eq!(root.screen_space_error, f64::MAX);
    }
}

/// Strict `<` SSE comparison plus `maximumLevel` clamping: raising
/// `maximumScreenSpaceError` above the roots' SSE stops refinement entirely.
#[test]
fn raising_maximum_screen_space_error_stops_refinement() {
    let mut quadtree = QuadtreePrimitive::new();
    quadtree.set_maximum_level(Some(1));
    let threshold = level_zero_threshold(&quadtree);
    let camera = camera_for_surface_distance(&quadtree, threshold * 0.5);

    // At threshold * 0.5 the SSE is twice the default maximum (2.0). Raising
    // the maximum to 4.1 satisfies `sse < maximumScreenSpaceError`.
    quadtree.set_maximum_screen_space_error(4.1);
    quadtree.update(&frame_state(camera));

    assert_eq!(quadtree.tiles_to_render().len(), 2);
    assert_eq!(quadtree.debug_tiles_visited, 2);
    for tile in quadtree.tiles_to_render() {
        assert_eq!(tile.level, 0);
    }
}
