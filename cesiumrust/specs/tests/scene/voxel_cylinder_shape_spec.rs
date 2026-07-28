//! VoxelCylinderShape tests ported from CesiumJS VoxelCylinderShapeSpec.js
//! Tests: constructs, update(modelMatrix/nonDefaultBounds/cross180), computeOBBForTile

use cesium_voxel::{VoxelCylinderShape, VoxelShape};
use glam::{DMat3, DMat4, DQuat, DVec3};

const EPSILON12: f64 = 1e-12;
const PI: f64 = std::f64::consts::PI;
const PI_OVER_TWO: f64 = std::f64::consts::FRAC_PI_2;
const PI_OVER_FOUR: f64 = std::f64::consts::FRAC_PI_4;

fn cylinder_default_min() -> DVec3 {
    DVec3::new(0.0, -PI, -1.0)
}
fn cylinder_default_max() -> DVec3 {
    DVec3::new(1.0, PI, 1.0)
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
        assert!(
            diff < EPSILON12,
            "{} col{}: {:?} != {:?} (diff={})",
            msg,
            i,
            a.col(i),
            b.col(i),
            diff
        );
    }
}

// ============================================================================
// constructs
// ============================================================================

#[test]
fn test_constructs() {
    // Ported from: "constructs"
    let shape = VoxelCylinderShape::new();
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
}

// ============================================================================
// update works with model matrix
// ============================================================================

#[test]
fn test_update_with_model_matrix() {
    // Ported from: "update works with model matrix"
    let mut shape = VoxelCylinderShape::new();

    let translation = DVec3::new(1.0, 2.0, 3.0);
    let scale = DVec3::new(2.0, 3.0, 4.0);
    let angle = PI_OVER_FOUR;
    let rotation = DQuat::from_axis_angle(DVec3::Z, angle);
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let visible = shape.update(
        model_matrix,
        cylinder_default_min(),
        cylinder_default_max(),
        None,
        None,
    );
    assert!(visible);

    // Expected OBB: center = translation
    // halfAxes = R(angle) * S(scale) (upper-left 3x3 of model matrix)
    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, translation, "OBB center");

    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let expected_half_axes = DMat3::from_cols(
        DVec3::new(scale.x * cos_a, scale.x * sin_a, 0.0),
        DVec3::new(
            scale.y * (angle + PI_OVER_TWO).cos(),
            scale.y * (angle + PI_OVER_TWO).sin(),
            0.0,
        ),
        DVec3::new(0.0, 0.0, scale.z),
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

    // boundTransform = fromRotationTranslation(halfAxes, center)
    let bt = shape.bound_transform();
    let bt_translation = bt.col(3).truncate();
    assert_vec3_eq(bt_translation, translation, "boundTransform translation");

    // shapeTransform = modelMatrix
    assert_eq!(shape.shape_transform(), model_matrix);
}

// ============================================================================
// update works with non-default minimum and maximum bounds
// ============================================================================

#[test]
fn test_update_with_non_default_bounds() {
    // Ported from: "update works with non-default minimum and maximum bounds"
    let mut shape = VoxelCylinderShape::new();

    let translation = DVec3::new(1.0, 2.0, 3.0);
    let scale = DVec3::new(2.0, 3.0, 4.0);
    let rotation = DQuat::IDENTITY;
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    // Half revolution
    let min_radius = 0.25;
    let max_radius = 0.75;
    let min_angle = -PI;
    let max_angle = 0.0;
    let min_height = -0.5;
    let max_height = 0.5;
    let min_bounds = DVec3::new(min_radius, min_angle, min_height);
    let max_bounds = DVec3::new(max_radius, max_angle, max_height);

    let visible = shape.update(model_matrix, min_bounds, max_bounds, None, None);
    assert!(visible);

    // Expected computation (from CesiumJS test):
    let expected_min_x = translation.x - max_radius * scale.x;
    let expected_max_x = translation.x + max_radius * scale.x;
    let expected_min_y = translation.y - max_radius * scale.y;
    let expected_max_y = translation.y;
    let expected_min_z = translation.z + min_height * scale.z;
    let expected_max_z = translation.z + max_height * scale.z;

    // x and y are swapped because scale is relative to angle midpoint: -pi/2
    let expected_scale = DVec3::new(
        0.5 * (expected_max_y - expected_min_y),
        0.5 * (expected_max_x - expected_min_x),
        0.5 * (expected_max_z - expected_min_z),
    );
    let expected_translation = DVec3::new(
        0.5 * (expected_max_x + expected_min_x),
        0.5 * (expected_max_y + expected_min_y),
        0.5 * (expected_max_z + expected_min_z),
    );

    // expectedRotation = Matrix3.fromRotationZ(-PI/2)
    let expected_rotation = DMat3::from_cols(
        DVec3::new(0.0, -1.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );
    let expected_half_axes = DMat3::from_cols(
        expected_rotation.col(0) * expected_scale.x,
        expected_rotation.col(1) * expected_scale.y,
        expected_rotation.col(2) * expected_scale.z,
    );

    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, expected_translation, "OBB center");
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

    // boundTransform
    let bt = shape.bound_transform();
    let bt_translation = bt.col(3).truncate();
    assert_vec3_eq(bt_translation, expected_translation, "boundTransform translation");

    // shapeTransform = modelMatrix
    assert_eq!(shape.shape_transform(), model_matrix);
}

// ============================================================================
// update works with bounds crossing the 180th meridian
// ============================================================================

#[test]
fn test_update_cross_180_meridian() {
    // Ported from: "update works with minimum and maximum bounds that cross the 180th meridian"
    let mut shape = VoxelCylinderShape::new();

    let translation = DVec3::ZERO;
    let scale = DVec3::ONE;
    let rotation = DQuat::IDENTITY;
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    // Half revolution around 180th meridian
    let min_angle = PI_OVER_TWO;
    let max_angle = -PI_OVER_TWO;
    let default_min = cylinder_default_min();
    let default_max = cylinder_default_max();
    let min_bounds = DVec3::new(default_min.x, min_angle, default_min.z);
    let max_bounds = DVec3::new(default_max.x, max_angle, default_max.z);

    let visible = shape.update(model_matrix, min_bounds, max_bounds, None, None);
    assert!(visible);

    // Expected (from CesiumJS test):
    let expected_scale = DVec3::new(0.5, 1.0, 1.0);
    let expected_translation = DVec3::new(-0.5, 0.0, 0.0);
    // expectedRotation = Matrix3.fromRotationZ(PI)
    let expected_rotation = DMat3::from_cols(
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, -1.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );
    let expected_half_axes = DMat3::from_cols(
        expected_rotation.col(0) * expected_scale.x,
        expected_rotation.col(1) * expected_scale.y,
        expected_rotation.col(2) * expected_scale.z,
    );

    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, expected_translation, "OBB center");
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

    // boundTransform
    let bt = shape.bound_transform();
    let bt_translation = bt.col(3).truncate();
    assert_vec3_eq(bt_translation, expected_translation, "boundTransform translation");

    // shapeTransform = modelMatrix
    assert_eq!(shape.shape_transform(), model_matrix);
}

// ============================================================================
// computeOrientedBoundingBoxForTile
// ============================================================================

#[test]
fn test_compute_obb_for_tile() {
    // Ported from: "computeOrientedBoundingBoxForTile returns oriented bounding box for a specified tile"
    let mut shape = VoxelCylinderShape::new();

    let translation = DVec3::new(1.0, 2.0, 3.0);
    let scale = DVec3::new(2.0, 3.0, 4.0);
    let rotation = DQuat::IDENTITY;
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    // Half revolution
    let min_radius = 0.25;
    let max_radius = 0.75;
    let min_angle = -PI;
    let max_angle = 0.0;
    let min_height = -0.5;
    let max_height = 0.5;
    let min_bounds = DVec3::new(min_radius, min_angle, min_height);
    let max_bounds = DVec3::new(max_radius, max_angle, max_height);
    shape.update(model_matrix, min_bounds, max_bounds, None, None);

    // Root tile (level=0, x=0, y=0, z=0)
    let tile_obb = shape.compute_obb_for_tile(0, 0, 0, 0);

    // Expected from CesiumJS test:
    // center = (1.0, 0.875, 3.0)
    // halfAxes = Matrix3(0, 1.5, 0, -1.125, 0, 0, 0, 0, 2) [row-major in CesiumJS]
    assert!(
        (tile_obb.center.x - 1.0).abs() < EPSILON12,
        "tile OBB center.x: {}",
        tile_obb.center.x
    );
    assert!(
        (tile_obb.center.y - 0.875).abs() < EPSILON12,
        "tile OBB center.y: {}",
        tile_obb.center.y
    );
    assert!(
        (tile_obb.center.z - 3.0).abs() < EPSILON12,
        "tile OBB center.z: {}",
        tile_obb.center.z
    );

    // CesiumJS Matrix3 row-major: (0, 1.5, 0, -1.125, 0, 0, 0, 0, 2)
    // Row-major means: row0=(0, 1.5, 0), row1=(-1.125, 0, 0), row2=(0, 0, 2)
    // In column-major (glam): col0=(0, -1.125, 0), col1=(1.5, 0, 0), col2=(0, 0, 2)
    let expected_half_axes = DMat3::from_cols(
        DVec3::new(0.0, -1.125, 0.0),
        DVec3::new(1.5, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 2.0),
    );
    assert_mat3_eq(tile_obb.half_axes, expected_half_axes, "tile OBB halfAxes");
}
