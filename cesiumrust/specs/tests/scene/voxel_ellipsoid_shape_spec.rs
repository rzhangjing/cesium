//! VoxelEllipsoidShape tests ported from CesiumJS VoxelEllipsoidShapeSpec.js
//! Tests: constructs, update visibility, OBB validity, compute_obb_for_tile

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

// ============================================================================
// constructs
// ============================================================================

#[test]
fn test_constructs() {
    // Ported from: "constructs"
    let shape = VoxelEllipsoidShape::new();
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
}

// ============================================================================
// update works with model matrix (visibility + basic OBB)
// ============================================================================

#[test]
fn test_update_with_model_matrix() {
    // Ported from: "update works with model matrix" (partial - visibility + OBB validity)
    let mut shape = VoxelEllipsoidShape::new();

    let translation = DVec3::new(1.0, 2.0, 3.0);
    let scale = DVec3::new(2.0, 2.0, 2.0);
    let angle = std::f64::consts::FRAC_PI_4;
    let rotation = DQuat::from_axis_angle(DVec3::Z, angle);
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let min_bounds = DVec3::new(-PI, -PI_OVER_TWO, 0.0);
    let max_bounds = DVec3::new(PI, PI_OVER_TWO, 100000.0);

    let visible = shape.update(model_matrix, min_bounds, max_bounds, None, None);
    assert!(visible);

    // OBB should have positive radius
    let obb = shape.oriented_bounding_box();
    assert!(obb.bounding_sphere_radius() > 0.0);

    // BoundingSphere should have positive radius
    let bs = shape.bounding_sphere();
    assert!(bs.radius > 0.0);

    // boundTransform translation should equal OBB center
    let bt = shape.bound_transform();
    let bt_translation = bt.col(3).truncate();
    assert!(
        (bt_translation - obb.center).length() < EPSILON12,
        "boundTransform translation should match OBB center"
    );
}

// ============================================================================
// update invisible when bounds are invalid
// ============================================================================

#[test]
fn test_update_invisible_clipped_away() {
    // Shape is invisible when clip bounds don't overlap
    let mut shape = VoxelEllipsoidShape::new();
    let visible = shape.update(
        DMat4::IDENTITY,
        ellipsoid_default_min(),
        ellipsoid_default_max(),
        Some(DVec3::new(5.0, 5.0, 5.0)),
        Some(DVec3::new(10.0, 10.0, 10.0)),
    );
    assert!(!visible);
}

// ============================================================================
// computeOrientedBoundingBoxForTile
// ============================================================================

#[test]
fn test_compute_obb_for_tile() {
    // Ported from: "computeOrientedBoundingBoxForTile returns oriented bounding box"
    // Uses unit sphere with height bounds [-0.5, 0.0]
    let mut shape = VoxelEllipsoidShape::with_radii(DVec3::ONE);

    let translation = DVec3::ZERO;
    let scale = DVec3::ONE;
    let rotation = DQuat::IDENTITY;
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let min_bounds = DVec3::new(-PI, -PI_OVER_TWO, -0.5);
    let max_bounds = DVec3::new(PI, PI_OVER_TWO, 0.0);
    let visible = shape.update(model_matrix, min_bounds, max_bounds, None, None);
    assert!(visible);

    // Root tile OBB should be valid
    let tile_obb = shape.compute_obb_for_tile(0, 0, 0, 0);
    assert!(
        tile_obb.bounding_sphere_radius() > 0.0,
        "tile OBB should have positive radius"
    );

    // Center should be near origin for full-sphere coverage with unit model matrix
    assert!(
        tile_obb.center.length() < 2.0,
        "tile OBB center should be near origin, got {:?}",
        tile_obb.center
    );
}

// ============================================================================
// update with default bounds produces valid OBB
// ============================================================================

#[test]
fn test_update_default_bounds() {
    // Full default bounds should produce a valid OBB
    let mut shape = VoxelEllipsoidShape::new();
    let visible = shape.update(
        DMat4::IDENTITY,
        ellipsoid_default_min(),
        ellipsoid_default_max(),
        None,
        None,
    );
    assert!(visible);

    let obb = shape.oriented_bounding_box();
    // WGS84 ellipsoid radius is ~6378137, OBB should encompass it
    assert!(
        obb.bounding_sphere_radius() > 6000000.0,
        "OBB radius should be > 6000000, got {}",
        obb.bounding_sphere_radius()
    );

    // shapeTransform should be identity
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
}
