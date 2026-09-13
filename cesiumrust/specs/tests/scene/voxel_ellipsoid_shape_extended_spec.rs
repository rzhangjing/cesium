//! VoxelEllipsoidShape extended tests — sampling, bounds, visibility edge cases
//! Additional ports from CesiumJS VoxelEllipsoidShapeSpec.js

use cesium_voxel::{VoxelEllipsoidShape, VoxelShape};
use glam::{DMat4, DQuat, DVec3};

const EPSILON12: f64 = 1e-12;
const PI: f64 = std::f64::consts::PI;
const PI_OVER_TWO: f64 = std::f64::consts::FRAC_PI_2;

fn ellipsoid_default_min() -> DVec3 {
    DVec3::new(-PI, -PI_OVER_TWO, -1.0)
}
fn ellipsoid_default_max() -> DVec3 {
    DVec3::new(PI, PI_OVER_TWO, 1.0)
}

fn assert_vec3_eq(a: DVec3, b: DVec3, msg: &str) {
    assert!(
        (a - b).length() < EPSILON12,
        "{}: {:?} != {:?} (diff={})",
        msg, a, b, (a - b).length()
    );
}

// ============================================================================
// Default state
// ============================================================================

#[test]
fn test_default_state() {
    let shape = VoxelEllipsoidShape::new();
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
    assert_eq!(shape.maximum_intersections_length(), 2);
}

#[test]
fn test_custom_radii() {
    let radii = DVec3::new(1.0, 2.0, 3.0);
    let shape = VoxelEllipsoidShape::with_radii(radii);
    assert!(shape.maximum_intersections_length() == 2);
}

// ============================================================================
// UV space transform
// ============================================================================

#[test]
fn test_convert_local_to_shape_uv_space_default() {
    let mut shape = VoxelEllipsoidShape::new();
    shape.update(DMat4::IDENTITY, ellipsoid_default_min(), ellipsoid_default_max(), None, None);

    // Min corner → (0, 0, 0)
    let uv = shape.convert_local_to_shape_uv_space(ellipsoid_default_min());
    assert!(uv.x.abs() < EPSILON12, "uv_min.x: {}", uv.x);
    assert!(uv.y.abs() < EPSILON12, "uv_min.y: {}", uv.y);
    assert!(uv.z.abs() < EPSILON12, "uv_min.z: {}", uv.z);

    // Max corner → (1, 1, 1)
    let uv = shape.convert_local_to_shape_uv_space(ellipsoid_default_max());
    assert!((uv.x - 1.0).abs() < EPSILON12, "uv_max.x: {}", uv.x);
    assert!((uv.y - 1.0).abs() < EPSILON12, "uv_max.y: {}", uv.y);
    assert!((uv.z - 1.0).abs() < EPSILON12, "uv_max.z: {}", uv.z);
}

#[test]
fn test_convert_local_to_shape_uv_space_north_america() {
    let mut shape = VoxelEllipsoidShape::new();
    let min_b = DVec3::new(-2.0, 0.4, -0.5);
    let max_b = DVec3::new(-1.0, 0.8, 0.5);
    shape.update(DMat4::IDENTITY, min_b, max_b, None, None);

    let uv = shape.convert_local_to_shape_uv_space(min_b);
    assert!(uv.x.abs() < EPSILON12);
    assert!(uv.y.abs() < EPSILON12);

    let uv = shape.convert_local_to_shape_uv_space(max_b);
    assert!((uv.x - 1.0).abs() < EPSILON12);
    assert!((uv.y - 1.0).abs() < EPSILON12);
}

// ============================================================================
// OBB tile computation
// ============================================================================

#[test]
fn test_compute_obb_for_tile_equator() {
    let mut shape = VoxelEllipsoidShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, ellipsoid_default_min(), ellipsoid_default_max(), None, None);

    // Level 1, tile (0, 0, 0) → longitude [-PI, 0], latitude [-PI/2, 0], height [-1, 0]
    let obb = shape.compute_obb_for_tile(1, 0, 0, 0);
    // Should be in the "southwest" octant near negative X
    assert!(obb.center.x < 0.0, "center.x should be negative");
}

#[test]
fn test_compute_obb_for_tile_north_pole() {
    let mut shape = VoxelEllipsoidShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, ellipsoid_default_min(), ellipsoid_default_max(), None, None);

    // Level 1, tile (0, 1, 0) → longitude [-PI, 0], latitude [0, PI/2], height [-1, 0]
    let obb = shape.compute_obb_for_tile(1, 0, 1, 0);
    // Should be in the "northwest" octant — center near negative X, positive Z
    assert!(obb.center.z > 0.0, "center.z should be positive for north");
}

// ============================================================================
// Visibility: zero scale
// ============================================================================

#[test]
fn test_invisible_zero_scale() {
    let mut shape = VoxelEllipsoidShape::new();
    let model = DMat4::from_scale(DVec3::new(0.0, 1.0, 1.0));
    let visible = shape.update(model, ellipsoid_default_min(), ellipsoid_default_max(), None, None);
    assert!(!visible, "Ellipsoid should be invisible when any scale component is zero");
}

#[test]
fn test_invisible_zero_scale_two_components() {
    let mut shape = VoxelEllipsoidShape::new();
    let model = DMat4::from_scale(DVec3::new(0.0, 0.0, 1.0));
    let visible = shape.update(model, ellipsoid_default_min(), ellipsoid_default_max(), None, None);
    assert!(!visible, "Ellipsoid should be invisible when multiple scale components are zero");
}

// ============================================================================
// Bounding sphere
// ============================================================================

#[test]
fn test_bounding_sphere_all_horizons() {
    let mut shape = VoxelEllipsoidShape::new();
    let model = DMat4::from_translation(DVec3::new(1000.0, 0.0, 0.0));
    shape.update(model, ellipsoid_default_min(), ellipsoid_default_max(), None, None);

    let bs = shape.bounding_sphere();
    assert!(bs.radius > 0.0);
}

// ============================================================================
// Update with rotation
// ============================================================================

#[test]
fn test_update_with_rotation() {
    let mut shape = VoxelEllipsoidShape::new();
    let translation = DVec3::new(100.0, 0.0, 0.0);
    let rotation = DQuat::from_axis_angle(DVec3::Z, PI / 3.0);
    let scale = DVec3::new(2.0, 1.0, 1.5);
    let model = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let visible = shape.update(model, ellipsoid_default_min(), ellipsoid_default_max(), None, None);
    assert!(visible);

    let obb = shape.oriented_bounding_box();
    // Center may not equal translation exactly for ellipsoid shape
    // Just verify non-zero and shape transform preserved
    assert!(obb.center.length() > 0.0);
    assert_eq!(shape.shape_transform(), model);
}

// ============================================================================
// bound_transform
// ============================================================================

#[test]
fn test_bound_transform_matches_obb() {
    let mut shape = VoxelEllipsoidShape::new();
    let model = DMat4::IDENTITY;
    shape.update(model, ellipsoid_default_min(), ellipsoid_default_max(), None, None);

    let bt = shape.bound_transform();
    let obb = shape.oriented_bounding_box();

    assert_vec3_eq(bt.col(3).truncate(), obb.center, "bound_transform center");
    for i in 0..3 {
        let bt_col = bt.col(i).truncate();
        let obb_col = obb.half_axes.col(i);
        assert!(
            (bt_col - obb_col).length() < EPSILON12,
            "bound_transform col{}: {:?} vs {:?}", i, bt_col, obb_col
        );
    }
}
