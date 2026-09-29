//! VoxelEllipsoidShape 测试，移植自 CesiumJS VoxelEllipsoidShapeSpec.js
//! 测试：constructs、update 可见性、OBB 有效性、compute_obb_for_tile

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
// constructs（构造）
// ============================================================================

#[test]
fn test_constructs() {
    // 移植自："constructs"
    let shape = VoxelEllipsoidShape::new();
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
}

// ============================================================================
// update 配合 model matrix 工作（可见性 + 基础 OBB）
// ============================================================================

#[test]
fn test_update_with_model_matrix() {
    // 移植自："update works with model matrix"（部分——可见性 + OBB 有效性）
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

    // OBB 半径应为正
    let obb = shape.oriented_bounding_box();
    assert!(obb.bounding_sphere_radius() > 0.0);

    // BoundingSphere 半径应为正
    let bs = shape.bounding_sphere();
    assert!(bs.radius > 0.0);

    // boundTransform 的平移应等于 OBB center
    let bt = shape.bound_transform();
    let bt_translation = bt.col(3).truncate();
    assert!(
        (bt_translation - obb.center).length() < EPSILON12,
        "boundTransform translation should match OBB center"
    );
}

// ============================================================================
// 边界非法时 update 不可见
// ============================================================================

#[test]
fn test_update_invisible_clipped_away() {
    // clip 边界不重叠时 shape 不可见
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
    // 移植自："computeOrientedBoundingBoxForTile returns oriented bounding box"
    // 使用单位球 + 高度边界 [-0.5, 0.0]
    let mut shape = VoxelEllipsoidShape::with_radii(DVec3::ONE);

    let translation = DVec3::ZERO;
    let scale = DVec3::ONE;
    let rotation = DQuat::IDENTITY;
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let min_bounds = DVec3::new(-PI, -PI_OVER_TWO, -0.5);
    let max_bounds = DVec3::new(PI, PI_OVER_TWO, 0.0);
    let visible = shape.update(model_matrix, min_bounds, max_bounds, None, None);
    assert!(visible);

    // 根瓦片 OBB 应有效
    let tile_obb = shape.compute_obb_for_tile(0, 0, 0, 0);
    assert!(
        tile_obb.bounding_sphere_radius() > 0.0,
        "tile OBB should have positive radius"
    );

    // 单位 model matrix 下全覆盖球体时，center 应靠近原点
    assert!(
        tile_obb.center.length() < 2.0,
        "tile OBB center should be near origin, got {:?}",
        tile_obb.center
    );
}

// ============================================================================
// 默认边界 update 产生有效 OBB
// ============================================================================

#[test]
fn test_update_default_bounds() {
    // 完整默认边界应产生有效 OBB
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
    // WGS84 椭球半径约 6378137，OBB 应包含它
    assert!(
        obb.bounding_sphere_radius() > 6000000.0,
        "OBB radius should be > 6000000, got {}",
        obb.bounding_sphere_radius()
    );

    // shapeTransform 应为单位阵
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
}
