//! VoxelBoxShape 扩展测试 — 采样、边界、OBB 边缘情形
//! 另从 CesiumJS VoxelBoxShapeSpec.js 移植

use cesium_voxel::{VoxelBoxShape, VoxelShape};
use glam::{DMat3, DMat4, DQuat, DVec3};

const EPSILON12: f64 = 1e-12;
const PI: f64 = std::f64::consts::PI;

fn default_min() -> DVec3 {
    DVec3::new(-1.0, -1.0, -1.0)
}
fn default_max() -> DVec3 {
    DVec3::new(1.0, 1.0, 1.0)
}

fn assert_vec3_eq(a: DVec3, b: DVec3, msg: &str) {
    assert!(
        (a - b).length() < EPSILON12,
        "{}: {:?} != {:?} (diff={})",
        msg, a, b, (a - b).length()
    );
}

// ============================================================================
// ConvertLocalToShapeUVSpace
// ============================================================================

#[test]
fn test_convert_local_to_shape_uv_space_identity() {
    let mut shape = VoxelBoxShape::new();
    shape.update(DMat4::IDENTITY, default_min(), default_max(), None, None);

    // 最小角点 → (0, 0, 0)
    let uv = shape.convert_local_to_shape_uv_space(default_min());
    assert!(uv.x.abs() < EPSILON12, "uv_min.x: {}", uv.x);
    assert!(uv.y.abs() < EPSILON12, "uv_min.y: {}", uv.y);
    assert!(uv.z.abs() < EPSILON12, "uv_min.z: {}", uv.z);

    // 最大角点 → (1, 1, 1)
    let uv = shape.convert_local_to_shape_uv_space(default_max());
    assert!((uv.x - 1.0).abs() < EPSILON12, "uv_max.x: {}", uv.x);
    assert!((uv.y - 1.0).abs() < EPSILON12, "uv_max.y: {}", uv.y);
    assert!((uv.z - 1.0).abs() < EPSILON12, "uv_max.z: {}", uv.z);

    // 中心 → (0.5, 0.5, 0.5)
    let uv = shape.convert_local_to_shape_uv_space(DVec3::ZERO);
    assert!((uv.x - 0.5).abs() < EPSILON12, "uv_center.x: {}", uv.x);
    assert!((uv.y - 0.5).abs() < EPSILON12, "uv_center.y: {}", uv.y);
    assert!((uv.z - 0.5).abs() < EPSILON12, "uv_center.z: {}", uv.z);
}

#[test]
fn test_convert_local_to_shape_uv_space_custom_bounds() {
    let mut shape = VoxelBoxShape::new();
    let min_b = DVec3::new(10.0, 20.0, 30.0);
    let max_b = DVec3::new(20.0, 40.0, 60.0);
    shape.update(DMat4::IDENTITY, min_b, max_b, None, None);

    // 最小角点 → (0, 0, 0)
    let uv = shape.convert_local_to_shape_uv_space(min_b);
    assert!(uv.x.abs() < EPSILON12);
    assert!(uv.y.abs() < EPSILON12);

    // 最大角点 → (1, 1, 1)
    let uv = shape.convert_local_to_shape_uv_space(max_b);
    assert!((uv.x - 1.0).abs() < EPSILON12);
    assert!((uv.y - 1.0).abs() < EPSILON12);

    // 中点 → (0.5, 0.5, 0.5)
    let mid = (min_b + max_b) * 0.5;
    let uv = shape.convert_local_to_shape_uv_space(mid);
    assert!((uv.x - 0.5).abs() < EPSILON12);
    assert!((uv.y - 0.5).abs() < EPSILON12);
}

// ============================================================================
// 带旋转的 OBB 性质
// ============================================================================

#[test]
fn test_obb_with_rotation_quaternion() {
    let mut shape = VoxelBoxShape::new();
    let translation = DVec3::new(5.0, 0.0, 0.0);
    let angle = PI / 3.0;
    let rotation = DQuat::from_axis_angle(DVec3::Z, angle);
    let scale = DVec3::new(2.0, 1.0, 3.0);
    let model_matrix = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    shape.update(model_matrix, default_min(), default_max(), None, None);

    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, translation, "OBB center with rotation");

    let bs = shape.bounding_sphere();
    assert!(bs.radius > 0.0);
    assert!((bs.radius - scale.length()).abs() < EPSILON12);
}

#[test]
fn test_obb_with_non_uniform_scale() {
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::from_scale(DVec3::new(0.5, 2.0, 3.0));

    shape.update(model_matrix, default_min(), default_max(), None, None);

    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, DVec3::ZERO, "OBB center");

    let expected_half_axes = DMat3::from_diagonal(DVec3::new(0.5, 2.0, 3.0));
    for i in 0..3 {
        let diff = (obb.half_axes.col(i) - expected_half_axes.col(i)).length();
        assert!(diff < EPSILON12, "half_axes col {}: diff={}", i, diff);
    }
}

// ============================================================================
// 包围球性质
// ============================================================================

#[test]
fn test_bounding_sphere_contains_obb_corners() {
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::from_translation(DVec3::new(10.0, 20.0, 30.0));
    shape.update(model_matrix, default_min(), default_max(), None, None);

    let bs = shape.bounding_sphere();
    let obb = shape.oriented_bounding_box();

    // OBB 的全部 8 个角点都应在包围球内部
    for sx in &[-1.0, 1.0f64] {
        for sy in &[-1.0, 1.0f64] {
            for sz in &[-1.0, 1.0f64] {
                let corner = obb.center
                    + obb.half_axes.col(0) * (*sx)
                    + obb.half_axes.col(1) * (*sy)
                    + obb.half_axes.col(2) * (*sz);
                let dist = (corner - bs.center).length();
                assert!(
                    dist <= bs.radius + EPSILON12,
                    "corner ({},{},{}) dist {} > radius {}",
                    sx, sy, sz, dist, bs.radius
                );
            }
        }
    }
}

// ============================================================================
// 更高层级下的 computeOrientedBoundingBoxForTile
// ============================================================================

#[test]
fn test_compute_obb_for_tile_level2() {
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, default_min(), default_max(), None, None);

    // 层级 2，瓦片 (1, 2, 1)
    let obb = shape.compute_obb_for_tile(2, 1, 2, 1);
    // x: -1 + 0.25*1 = -0.75, but using lerp: -1 + 0.25*(tile_x+1)*2 = -1+0.5 = -0.5
    // Actually: tile_min.x = lerp(-1, 1, 0.25 * 1) = lerp(-1, 1, 0.25) = -1 + 0.5 = -0.5
    // tile_max.x = lerp(-1, 1, 0.25 * 2) = lerp(-1, 1, 0.5) = 0
    // center.x = (-0.5 + 0) / 2 = -0.25
    // tile_min.y = lerp(-1, 1, 0.25 * 2) = lerp(-1, 1, 0.5) = 0
    // tile_max.y = lerp(-1, 1, 0.25 * 3) = lerp(-1, 1, 0.75) = 0.5
    // center.y = (0 + 0.5) / 2 = 0.25
    assert!((obb.center.x + 0.25).abs() < EPSILON12, "center.x: {}", obb.center.x);
    assert!((obb.center.y - 0.25).abs() < EPSILON12, "center.y: {}", obb.center.y);
    assert!((obb.center.z + 0.25).abs() < EPSILON12, "center.z: {}", obb.center.z);
    // 对层级 2，halfAxes 在对角线上应为 0.25
    let expected_half_axes = DMat3::from_diagonal(DVec3::new(0.25, 0.25, 0.25));
    for i in 0..3 {
        let diff = (obb.half_axes.col(i) - expected_half_axes.col(i)).length();
        assert!(diff < EPSILON12, "level2 half_axes col{}", i);
    }
}

#[test]
fn test_compute_obb_for_tile_level3_corner() {
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, default_min(), default_max(), None, None);

    // 层级 3，瓦片 (7, 7, 7) — 最后一个角点
    let obb = shape.compute_obb_for_tile(3, 7, 7, 7);
    // 应在靠近 (1, 1, 1) 的 (+, +, +) 卦限内
    assert!(obb.center.x > 0.5);
    assert!(obb.center.y > 0.5);
    assert!(obb.center.z > 0.5);
}

// ============================================================================
// 使用裁剪边界更新
// ============================================================================

#[test]
fn test_update_clipped_bounds_half_box() {
    let mut shape = VoxelBoxShape::new();
    let model_matrix = DMat4::IDENTITY;

    // 裁剪至第一卦限：[0, 1]^3
    let visible = shape.update(
        model_matrix,
        default_min(),
        default_max(),
        Some(DVec3::new(0.0, 0.0, 0.0)),
        Some(DVec3::new(1.0, 1.0, 1.0)),
    );
    assert!(visible);

    // OBB 中心应位于 (0.5, 0.5, 0.5)
    let obb = shape.oriented_bounding_box();
    assert_vec3_eq(obb.center, DVec3::new(0.5, 0.5, 0.5), "clipped center");

    // 各轴的半轴长应为 0.5
    let expected_half = DMat3::from_diagonal(DVec3::new(0.5, 0.5, 0.5));
    for i in 0..3 {
        let diff = (obb.half_axes.col(i) - expected_half.col(i)).length();
        assert!(diff < EPSILON12, "clipped half_axes col{}", i);
    }
}

// ============================================================================
// 模型矩阵下 UV 变换的不变性
// ============================================================================

#[test]
fn test_uv_transform_invariant_under_translation() {
    let mut shape = VoxelBoxShape::new();
    let model = DMat4::from_translation(DVec3::new(100.0, 200.0, 300.0));
    shape.update(model, default_min(), default_max(), None, None);

    // UV 坐标应独立于模型平移
    let uv = shape.convert_local_to_shape_uv_space(DVec3::ZERO);
    assert!((uv.x - 0.5).abs() < EPSILON12, "UV x invariant: {}", uv.x);
    assert!((uv.y - 0.5).abs() < EPSILON12, "UV y invariant: {}", uv.y);
}

// ============================================================================
// 裁剪后包含点
// ============================================================================

#[test]
fn test_contains_local_respects_clipping() {
    let mut shape = VoxelBoxShape::new();
    shape.update(
        DMat4::IDENTITY,
        default_min(),
        default_max(),
        Some(DVec3::new(0.0, 0.0, 0.0)),
        Some(DVec3::new(0.5, 0.5, 0.5)),
    );

    assert!(shape.contains_local(DVec3::new(0.1, 0.1, 0.1)));
    assert!(!shape.contains_local(DVec3::new(-0.1, 0.1, 0.1)));
    assert!(!shape.contains_local(DVec3::new(0.6, 0.1, 0.1)));
}

// ============================================================================
// bound_transform 正确性
// ============================================================================

#[test]
fn test_bound_transform_encodes_obb() {
    let mut shape = VoxelBoxShape::new();
    let model = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
    shape.update(model, default_min(), default_max(), None, None);

    let bt = shape.bound_transform();
    let obb = shape.oriented_bounding_box();

    // bound_transform 的平移部分应为 OBB 中心
    assert_vec3_eq(bt.col(3).truncate(), obb.center, "bound_transform translation");

    // 左上 3x3 应为 OBB half_axes
    for i in 0..3 {
        let bt_col = bt.col(i).truncate();
        let obb_col = obb.half_axes.col(i);
        assert!(
            (bt_col - obb_col).length() < EPSILON12,
            "bound_transform align col{}: {:?} vs {:?}",
            i, bt_col, obb_col
        );
    }
}

// ============================================================================
// 最大相交数
// ============================================================================

#[test]
fn test_maximum_intersections_default() {
    let shape = VoxelBoxShape::new();
    assert_eq!(shape.maximum_intersections_length(), 1);
}

#[test]
fn test_maximum_intersections_after_update() {
    let mut shape = VoxelBoxShape::new();
    shape.update(DMat4::IDENTITY, default_min(), default_max(), None, None);
    assert_eq!(shape.maximum_intersections_length(), 1);
}
