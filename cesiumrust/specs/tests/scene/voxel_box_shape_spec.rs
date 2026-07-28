//! VoxelBoxShape tests ported from CesiumJS VoxelBoxShapeSpec.js
//! Tests: constructs, update(modelMatrix/bounds/visibility), computeOBBForTile

use cesium_voxel::{VoxelBoxShape, VoxelShape};
use glam::{DMat3, DMat4, DQuat, DVec3};

const EPSILON12: f64 = 1e-12;

fn default_min() -> DVec3 {
    DVec3::new(-1.0, -1.0, -1.0)
}
fn default_max() -> DVec3 {
    DVec3::new(1.0, 1.0, 1.0)
}

fn assert_vec3_eq(a: DVec3, b: DVec3, msg: &str) {
    assert!(
        (a - b).length() < EPSILON12,
        "{}: {:?} != {:?}",
        msg,
        a,
        b
    );
}

fn assert_mat3_eq(a: DMat3, b: DMat3, msg: &str) {
    for i in 0..3 {
        let diff = (a.col(i) - b.col(i)).length();
        assert!(diff < EPSILON12, "{} col{}: {:?} != {:?}", msg, i, a.col(i), b.col(i));
    }
}

// ============================================================================
// constructs
// ============================================================================

#[test]
fn test_constructs() {
    // Ported from: "constructs"
    let shape = VoxelBoxShape::new();
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
}

// ============================================================================
// update works with model matrix
// ============================================================================

#[test]
fn test_update_with_model_matrix() {
    // Ported from: "update works with model matrix"
    let mut shape = VoxelBoxShape::new();

    let translation = DVec3::new(1.0, 2.0, 3.0);
    let scale = DVec3::new(2.0, 3.0, 4.0);
    let angle = std::f64::consts::FRAC_PI_4;
    let rotation = DQuat::from_axis_angle(DVec3::Z, angle);

    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let visible = shape.update(model_matrix, default_min(), default_max(), None, None);
    assert!(visible);

    // Expected OBB: center = translation, halfAxes = upper-left 3x3 of model matrix
    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, translation, "OBB center");

    // For default bounds, halfAxes = Matrix4.getMatrix3(modelMatrix) = R*S
    let expected_half_axes = DMat3::from_cols(
        model_matrix.col(0).truncate(),
        model_matrix.col(1).truncate(),
        model_matrix.col(2).truncate(),
    );
    assert_mat3_eq(obb.half_axes, expected_half_axes, "OBB halfAxes");

    // BoundingSphere: center = translation, radius = |scale|
    let bs = shape.bounding_sphere();
    assert_vec3_eq(bs.center, translation, "BS center");
    let expected_radius = scale.length();
    assert!(
        (bs.radius - expected_radius).abs() < EPSILON12,
        "BS radius: {} != {}",
        bs.radius,
        expected_radius
    );

    // boundTransform and shapeTransform
    assert_eq!(shape.shape_transform(), model_matrix);
}

// ============================================================================
// update works with non-default bounds
// ============================================================================

#[test]
fn test_update_with_non_default_bounds() {
    // Ported from: "update works with non-default minimum and maximum bounds"
    let mut shape = VoxelBoxShape::new();

    let translation = DVec3::new(1.0, 2.0, 3.0);
    let scale = DVec3::new(2.0, 3.0, 4.0);
    let rotation = DQuat::IDENTITY;
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let min_bounds = DVec3::new(-0.75, -0.75, -0.75);
    let max_bounds = DVec3::new(-0.25, -0.25, -0.25);

    let visible = shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(min_bounds),
        Some(max_bounds),
    );
    assert!(visible);

    // Expected: localCenter = midpoint(-0.75, -0.25) = (-0.5, -0.5, -0.5)
    // center = modelMatrix * localCenter = S*localCenter + T
    let expected_translation = DVec3::new(0.0, 0.5, 1.0);
    // scale_half = (2*0.5*0.5, 3*0.5*0.5, 4*0.5*0.5) = (0.5, 0.75, 1.0)
    let expected_scale = DVec3::new(0.5, 0.75, 1.0);

    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, expected_translation, "OBB center");

    let expected_half_axes = DMat3::from_diagonal(expected_scale);
    assert_mat3_eq(obb.half_axes, expected_half_axes, "OBB halfAxes");

    // BoundingSphere
    let bs = shape.bounding_sphere();
    assert_vec3_eq(bs.center, expected_translation, "BS center");
    let expected_radius = expected_scale.length();
    assert!(
        (bs.radius - expected_radius).abs() < EPSILON12,
        "BS radius: {} != {}",
        bs.radius,
        expected_radius
    );

    // shapeTransform = original modelMatrix
    assert_eq!(shape.shape_transform(), model_matrix);
}

// ============================================================================
// update visibility: zero scale
// ============================================================================

#[test]
fn test_update_invisible_zero_scale_two_or_more() {
    // Ported from: "update is invisible with zero scale for two or more components"
    let mut shape = VoxelBoxShape::new();
    let translation = DVec3::new(1.0, 2.0, 3.0);
    let rotation = DQuat::IDENTITY;

    // 0 scale for X and Y
    let scale = DVec3::new(0.0, 0.0, 2.0);
    let mm = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    assert!(!shape.update(mm, default_min(), default_max(), None, None));

    // 0 scale for X and Z
    let scale = DVec3::new(0.0, 2.0, 0.0);
    let mm = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    assert!(!shape.update(mm, default_min(), default_max(), None, None));

    // 0 scale for Y and Z
    let scale = DVec3::new(2.0, 0.0, 0.0);
    let mm = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    assert!(!shape.update(mm, default_min(), default_max(), None, None));

    // 0 scale for X, Y, and Z
    let scale = DVec3::new(0.0, 0.0, 0.0);
    let mm = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    assert!(!shape.update(mm, default_min(), default_max(), None, None));
}

#[test]
fn test_update_invisible_zero_scale_single() {
    // CesiumJS: ANY zero scale → invisible (comment: "too annoying to reconstruct rotation matrix")
    let mut shape = VoxelBoxShape::new();
    let translation = DVec3::new(1.0, 2.0, 3.0);
    let rotation = DQuat::IDENTITY;

    // 0 scale for X only → still invisible
    let scale = DVec3::new(0.0, 2.0, 2.0);
    let mm = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    assert!(!shape.update(mm, default_min(), default_max(), None, None));

    // 0 scale for Y only
    let scale = DVec3::new(2.0, 0.0, 2.0);
    let mm = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    assert!(!shape.update(mm, default_min(), default_max(), None, None));

    // 0 scale for Z only
    let scale = DVec3::new(2.0, 2.0, 0.0);
    let mm = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    assert!(!shape.update(mm, default_min(), default_max(), None, None));
}

// ============================================================================
// update visibility: zero bounds
// ============================================================================

#[test]
fn test_update_visible_zero_bounds_one_component() {
    // Ported from: "update is visible with zero bounds for one component"
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;
    let clip_min = DVec3::new(-1.0, -1.0, -1.0);
    let clip_max = DVec3::new(1.0, 1.0, 1.0);

    // 0 in X bound
    let min_bounds = DVec3::new(0.0, -1.0, -1.0);
    let max_bounds = DVec3::new(0.0, 1.0, 1.0);
    let visible = shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(clip_min),
        Some(clip_max),
    );
    assert!(visible, "zero X bound should be visible");

    // 0 in Y bound
    let min_bounds = DVec3::new(-1.0, 0.0, -1.0);
    let max_bounds = DVec3::new(1.0, 0.0, 1.0);
    let visible = shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(clip_min),
        Some(clip_max),
    );
    assert!(visible, "zero Y bound should be visible");

    // 0 in Z bound
    let min_bounds = DVec3::new(-1.0, -1.0, 0.0);
    let max_bounds = DVec3::new(1.0, 1.0, 0.0);
    let visible = shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(clip_min),
        Some(clip_max),
    );
    assert!(visible, "zero Z bound should be visible");
}

#[test]
fn test_update_invisible_zero_bounds_two_or_more() {
    // Ported from: "update is invisible with zero bounds for two or more components"
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;

    // 0 in X and Y bounds
    let min_bounds = DVec3::new(0.0, 0.0, -1.0);
    let max_bounds = DVec3::new(0.0, 0.0, 1.0);
    assert!(!shape.update(model_matrix, min_bounds, max_bounds, None, None));

    // 0 in X and Z bounds
    let min_bounds = DVec3::new(0.0, -1.0, 0.0);
    let max_bounds = DVec3::new(0.0, 1.0, 0.0);
    assert!(!shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(min_bounds),
        Some(max_bounds)
    ));

    // 0 in Y and Z bounds
    let min_bounds = DVec3::new(-1.0, 0.0, 0.0);
    let max_bounds = DVec3::new(1.0, 0.0, 0.0);
    assert!(!shape.update(model_matrix, min_bounds, max_bounds, None, None));

    // 0 in X, Y, and Z bounds
    let min_bounds = DVec3::new(0.0, 0.0, 0.0);
    let max_bounds = DVec3::new(0.0, 0.0, 0.0);
    assert!(!shape.update(model_matrix, min_bounds, max_bounds, None, None));
}

// ============================================================================
// update visibility: min bounds exceed max bounds
// ============================================================================

#[test]
fn test_update_invisible_min_exceeds_max() {
    // Ported from: "update is invisible when minimum bounds exceed maximum bounds"
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;
    let clip_min = DVec3::new(-1.0, -1.0, -1.0);
    let clip_max = DVec3::new(2.0, 2.0, 2.0);

    // Exceeds X
    let min_bounds = DVec3::new(1.0, -1.0, -1.0);
    let max_bounds = DVec3::new(0.9, 1.0, 1.0);
    assert!(!shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(clip_min),
        Some(clip_max)
    ));

    // Exceeds Y
    let min_bounds = DVec3::new(-1.0, 1.0, -1.0);
    let max_bounds = DVec3::new(1.0, 0.9, 1.0);
    assert!(!shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(clip_min),
        Some(clip_max)
    ));

    // Exceeds Z
    let min_bounds = DVec3::new(-1.0, -1.0, 1.0);
    let max_bounds = DVec3::new(1.0, 1.0, 0.9);
    assert!(!shape.update(
        model_matrix,
        min_bounds,
        max_bounds,
        Some(clip_min),
        Some(clip_max)
    ));
}

// ============================================================================
// computeOrientedBoundingBoxForTile
// ============================================================================

#[test]
fn test_compute_obb_for_tile_root() {
    // Ported from: "computeOrientedBoundingBoxForTile works for root tile"
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, default_min(), default_max(), None, None);

    let tile_obb = shape.compute_obb_for_tile(0, 0, 0, 0);
    let shape_obb = shape.oriented_bounding_box();

    assert_vec3_eq(tile_obb.center, shape_obb.center, "root tile OBB center");
    assert_mat3_eq(tile_obb.half_axes, shape_obb.half_axes, "root tile OBB halfAxes");
}

#[test]
fn test_compute_obb_for_tile_children() {
    // Ported from: "computeOrientedBoundingBoxForTile works for children of root tile"
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, default_min(), default_max(), None, None);

    let expected_scale = DVec3::new(0.5, 0.5, 0.5);
    let expected_half_axes = DMat3::from_diagonal(expected_scale);

    // Child (0, 0, 0)
    let obb = shape.compute_obb_for_tile(1, 0, 0, 0);
    assert_vec3_eq(obb.center, DVec3::new(-0.5, -0.5, -0.5), "child(0,0,0) center");
    assert_mat3_eq(obb.half_axes, expected_half_axes, "child(0,0,0) halfAxes");

    // Child (1, 0, 0)
    let obb = shape.compute_obb_for_tile(1, 1, 0, 0);
    assert_vec3_eq(obb.center, DVec3::new(0.5, -0.5, -0.5), "child(1,0,0) center");
    assert_mat3_eq(obb.half_axes, expected_half_axes, "child(1,0,0) halfAxes");

    // Child (0, 1, 0)
    let obb = shape.compute_obb_for_tile(1, 0, 1, 0);
    assert_vec3_eq(obb.center, DVec3::new(-0.5, 0.5, -0.5), "child(0,1,0) center");
    assert_mat3_eq(obb.half_axes, expected_half_axes, "child(0,1,0) halfAxes");

    // Child (0, 0, 1)
    let obb = shape.compute_obb_for_tile(1, 0, 0, 1);
    assert_vec3_eq(obb.center, DVec3::new(-0.5, -0.5, 0.5), "child(0,0,1) center");
    assert_mat3_eq(obb.half_axes, expected_half_axes, "child(0,0,1) halfAxes");

    // Child (1, 1, 0)
    let obb = shape.compute_obb_for_tile(1, 1, 1, 0);
    assert_vec3_eq(obb.center, DVec3::new(0.5, 0.5, -0.5), "child(1,1,0) center");
    assert_mat3_eq(obb.half_axes, expected_half_axes, "child(1,1,0) halfAxes");

    // Child (1, 0, 1)
    let obb = shape.compute_obb_for_tile(1, 1, 0, 1);
    assert_vec3_eq(obb.center, DVec3::new(0.5, -0.5, 0.5), "child(1,0,1) center");
    assert_mat3_eq(obb.half_axes, expected_half_axes, "child(1,0,1) halfAxes");

    // Child (1, 1, 1)
    let obb = shape.compute_obb_for_tile(1, 1, 1, 1);
    assert_vec3_eq(obb.center, DVec3::new(0.5, 0.5, 0.5), "child(1,1,1) center");
    assert_mat3_eq(obb.half_axes, expected_half_axes, "child(1,1,1) halfAxes");
}
