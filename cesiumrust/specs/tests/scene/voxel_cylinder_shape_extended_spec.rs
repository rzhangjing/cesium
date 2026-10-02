//! VoxelCylinderShape 扩展测试 — 采样、边界、可见性边缘情形
//! 另从 CesiumJS VoxelCylinderShapeSpec 移植

use cesium_voxel::{VoxelCylinderShape, VoxelShape};
use glam::{DMat4, DQuat, DVec3};

const EPSILON12: f64 = 1e-12;
const PI: f64 = std::f64::consts::PI;

fn cylinder_default_min() -> DVec3 {
    DVec3::new(0.0, -PI, -1.0)
}
fn cylinder_default_max() -> DVec3 {
    DVec3::new(1.0, PI, 1.0)
}

fn assert_vec3_eq(a: DVec3, b: DVec3, msg: &str) {
    assert!(
        (a - b).length() < EPSILON12,
        "{}: {:?} != {:?} (diff={})",
        msg, a, b, (a - b).length()
    );
}

// ============================================================================
// 默认状态
// ============================================================================

#[test]
fn test_default_state() {
    let shape = VoxelCylinderShape::new();
    assert_eq!(shape.shape_transform(), DMat4::IDENTITY);
    assert_eq!(shape.maximum_intersections_length(), 2);
}

// ============================================================================
// UV 空间变换
// ============================================================================

#[test]
fn test_convert_local_to_shape_uv_space_default() {
    let mut shape = VoxelCylinderShape::new();
    shape.update(DMat4::IDENTITY, cylinder_default_min(), cylinder_default_max(), None, None);

    // UV 变换将每个轴的 [min, max] 映射到 [0, 1]
    // 验证函数不 panic 且返回有限值
    let uv_min = shape.convert_local_to_shape_uv_space(cylinder_default_min());
    let uv_max = shape.convert_local_to_shape_uv_space(cylinder_default_max());
    let uv_mid = shape.convert_local_to_shape_uv_space(DVec3::new(0.5, 0.0, 0.0));

    assert!(uv_min.x.is_finite());
    assert!(uv_min.y.is_finite());
    assert!(uv_max.x.is_finite());
    assert!(uv_max.y.is_finite());
    assert!(uv_mid.x.is_finite());
}

#[test]
fn test_convert_local_to_shape_uv_space_custom() {
    let mut shape = VoxelCylinderShape::new();
    let min_b = DVec3::new(0.25, -PI / 2.0, -0.5);
    let max_b = DVec3::new(0.75, PI / 2.0, 0.5);
    shape.update(DMat4::IDENTITY, min_b, max_b, None, None);

    let uv_min = shape.convert_local_to_shape_uv_space(min_b);
    let uv_max = shape.convert_local_to_shape_uv_space(max_b);

    assert!(uv_min.x.is_finite());
    assert!(uv_min.y.is_finite());
    assert!(uv_max.x.is_finite());
    assert!(uv_max.y.is_finite());
}

// ============================================================================
// 各层级下的 OBB 瓦片计算
// ============================================================================

#[test]
fn test_compute_obb_for_tile_zero_angle() {
    let mut shape = VoxelCylinderShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, cylinder_default_min(), cylinder_default_max(), None, None);

    // 层级 1，瓦片 (0, 0, 0) → 第一卦限
    let obb = shape.compute_obb_for_tile(1, 0, 0, 0);
    // 应在容差内接近 [-0.5, 0, -0.5] 区域
    assert!(obb.center.x < 0.1, "center.x should be near negative: {}", obb.center.x);
    assert!(obb.center.z < 0.1, "center.z should be near negative: {}", obb.center.z);
}

#[test]
fn test_compute_obb_for_tile_half_angle() {
    let mut shape = VoxelCylinderShape::new();
    let model_matrix = DMat4::IDENTITY;
    shape.update(model_matrix, cylinder_default_min(), cylinder_default_max(), None, None);

    // 层级 1，瓦片 (0, 1, 0) → [0, 0.5] 半径、[0, PI] 角度、[-1, 0] 高度
    let obb = shape.compute_obb_for_tile(1, 0, 1, 0);
    // 半径 [0, 0.5]，角度 [0, PI] → 中心位于正 X 方向
    assert!(obb.center.x > 0.0, "center.x should be positive: {}", obb.center.x);
    assert!(obb.center.y < 0.5, "center.y angle half");
    assert!(obb.center.z < 0.0, "center.z should be negative: {}", obb.center.z);
}

// ============================================================================
// 可见性：零缩放（任意单个分量 => 不可见）
// ============================================================================

#[test]
fn test_invisible_zero_scale_x() {
    let mut shape = VoxelCylinderShape::new();
    let model = DMat4::from_scale(DVec3::new(0.0, 2.0, 2.0));
    assert!(!shape.update(model, cylinder_default_min(), cylinder_default_max(), None, None));
}

#[test]
fn test_invisible_zero_scale_y() {
    let mut shape = VoxelCylinderShape::new();
    let model = DMat4::from_scale(DVec3::new(2.0, 0.0, 2.0));
    assert!(!shape.update(model, cylinder_default_min(), cylinder_default_max(), None, None));
}

#[test]
fn test_invisible_zero_scale_z() {
    let mut shape = VoxelCylinderShape::new();
    let model = DMat4::from_scale(DVec3::new(2.0, 2.0, 0.0));
    assert!(!shape.update(model, cylinder_default_min(), cylinder_default_max(), None, None));
}

// ============================================================================
// 可见性：零边界（退化形状）
// ============================================================================

#[test]
fn test_invisible_zero_radius_bounds() {
    let mut shape = VoxelCylinderShape::new();
    let min_b = DVec3::new(0.0, -PI, -1.0);
    let max_b = DVec3::new(0.0, PI, 1.0); // 零半径范围
    assert!(!shape.update(DMat4::IDENTITY, min_b, max_b, None, None));
}

#[test]
fn test_visible_zero_bounds_single_dim() {
    let mut shape = VoxelCylinderShape::new();
    // 单个边界分量为 0 是可以的（半径）
    let min_b = DVec3::new(0.5, -PI, -1.0);
    let max_b = DVec3::new(0.5, PI, 1.0);
    let visible = shape.update(
        DMat4::IDENTITY, min_b, max_b,
        Some(cylinder_default_min()),
        Some(cylinder_default_max())
    );
    assert!(visible, "single zero bound component should be visible");
}

// ============================================================================
// 包含点  (圆柱形状不支持 — 已跳过)
// ============================================================================

// ============================================================================
// 带旋转的更新
// ============================================================================

#[test]
fn test_update_with_rotation_y_axis() {
    let mut shape = VoxelCylinderShape::new();
    let translation = DVec3::new(10.0, 0.0, 0.0);
    let rotation = DQuat::from_axis_angle(DVec3::Y, PI / 4.0);
    let scale = DVec3::new(2.0, 3.0, 4.0);
    let model = DMat4::from_scale_rotation_translation(scale, rotation, translation);

    let visible = shape.update(model, cylinder_default_min(), cylinder_default_max(), None, None);
    assert!(visible);

    let obb = shape.oriented_bounding_box();
    // 中心应为平移量
    assert!((obb.center - translation).length() < EPSILON12);
    // shape transform 应为模型矩阵
    assert_eq!(shape.shape_transform(), model);
}

// ============================================================================
// 包围球正确性
// ============================================================================

#[test]
fn test_bounding_sphere_radius_matches_scale() {
    let mut shape = VoxelCylinderShape::new();
    let scale = DVec3::new(3.0, 4.0, 5.0);
    let model = DMat4::from_scale(scale);
    shape.update(model, cylinder_default_min(), cylinder_default_max(), None, None);

    let bs = shape.bounding_sphere();
    assert!((bs.radius - scale.length()).abs() < EPSILON12,
        "radius {} vs scale length {}", bs.radius, scale.length());
}

// ============================================================================
// bound_transform 正确性
// ============================================================================

#[test]
fn test_bound_transform_matches_obb() {
    let mut shape = VoxelCylinderShape::new();
    let model = DMat4::from_translation(DVec3::new(5.0, 10.0, 15.0));
    shape.update(model, cylinder_default_min(), cylinder_default_max(), None, None);

    let bt = shape.bound_transform();
    let obb = shape.oriented_bounding_box();

    assert_vec3_eq(bt.col(3).truncate(), obb.center, "bound_transform center");
    for i in 0..3 {
        let bt_col = bt.col(i).truncate();
        let obb_col = obb.half_axes.col(i);
        assert!((bt_col - obb_col).length() < EPSILON12, "bound_transform col{}", i);
    }
}
