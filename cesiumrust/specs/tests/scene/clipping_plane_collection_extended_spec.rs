//! ClippingPlaneCollection 扩展 specs — transform、状态、边缘情形
//! 另从 CesiumJS Scene/ClippingPlaneCollectionSpec.js 移植

use cesium_effects::{ClippingPlane, ClippingPlaneCollection, Intersect};
use glam::{DMat4, DVec3};
use std::f64::consts::PI;

const EPSILON10: f64 = 1e-10;

// ============================================================================
// ClippingPlane: 带旋转的 transform
// ============================================================================

#[test]
fn plane_transform_rotation_90_y() {
    let plane = ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0);
    // 绕 Y 旋转 90°
    let rotation = DMat4::from_rotation_y(PI / 2.0);
    let transformed = plane.transform(&rotation);

    // X 轴绕 Y 旋转 90° 后，法线应为 (0, 0, -1)
    assert!((transformed.normal.z + 1.0).abs() < EPSILON10,
        "normal.z should be -1: {:?}", transformed.normal);
    assert!(transformed.normal.x.abs() < EPSILON10);
    assert!(transformed.normal.y.abs() < EPSILON10);
}

#[test]
fn plane_transform_scale_uniform() {
    let plane = ClippingPlane::new(DVec3::X, 5.0);
    let scale = DMat4::from_scale(DVec3::new(2.0, 2.0, 2.0));
    let transformed = plane.transform(&scale);

    // 法线应保持（单位长度）
    assert!((transformed.normal.length() - 1.0).abs() < EPSILON10);
    // 距离应翻倍（均匀缩放 2）
    assert!((transformed.distance - 10.0).abs() < EPSILON10,
        "distance: {}", transformed.distance);
}

#[test]
fn plane_transform_translation_2() {
    let plane = ClippingPlane::new(DVec3::X, 0.0); // 位于 x=0 的平面
    let translation = DMat4::from_translation(DVec3::new(10.0, 0.0, 0.0));
    let transformed = plane.transform(&translation);

    // 法线不变
    assert!((transformed.normal - DVec3::X).length() < EPSILON10);
    // 距离应为 -10（局部中 x=0 的平面 → 世界中 x=10，但 transform 应用于平面坐标系）
    assert!((transformed.distance + 10.0).abs() < EPSILON10,
        "distance: {}", transformed.distance);
}

#[test]
fn plane_transform_chain() {
    let plane = ClippingPlane::new(DVec3::Y, 0.0);
    let translate = DMat4::from_translation(DVec3::new(0.0, 5.0, 0.0));
    let scale = DMat4::from_scale(DVec3::new(2.0, 2.0, 2.0));

    let _t1 = plane.transform(&translate);
    let t2 = plane.transform(&(scale * translate));

    // 平移+缩放后：法线应同向，距离被缩放
    assert!((t2.normal.length() - 1.0).abs() < EPSILON10);
}

// ============================================================================
// ClippingPlane: 边缘情形
// ============================================================================

#[test]
fn plane_negative_normal_is_inside() {
    // 法线为 -X、距离为 5 的平面意味着：-dot(X, point) + 5 >= 0 → x <= 5
    let plane = ClippingPlane::new(DVec3::new(-1.0, 0.0, 0.0), 5.0);
    assert!(plane.is_inside(DVec3::new(0.0, 0.0, 0.0)));
    assert!(plane.is_inside(DVec3::new(4.0, 0.0, 0.0)));
    assert!(!plane.is_inside(DVec3::new(6.0, 0.0, 0.0)));
}

#[test]
fn plane_to_vec4_preserves_semantics() {
    let plane = ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), -3.0);
    let v = plane.to_vec4();

    assert!((v[1] - 1.0).abs() < EPSILON10);
    assert!((v[3] - plane.distance).abs() < EPSILON10);
}

// ============================================================================
// ClippingPlaneCollection: get_mut
// ============================================================================

#[test]
fn collection_get_mut_modifies() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::Y, 0.0),
    ]);

    if let Some(plane) = collection.get_mut(0) {
        plane.distance = 10.0;
    }

    let modified = collection.get(0).unwrap();
    assert!((modified.distance - 10.0).abs() < EPSILON10);
}

#[test]
fn collection_get_mut_out_of_bounds() {
    let mut collection = ClippingPlaneCollection::new();
    assert!(collection.get_mut(0).is_none());
    assert!(collection.get_mut(999).is_none());
}

// ============================================================================
// ClippingPlaneCollection: model_matrix
// ============================================================================

#[test]
fn collection_with_model_matrix_clips_correctly() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::Y, 0.0), // 在局部空间中保留 y >= 0
    ]);
    // 模型矩阵：将坐标系沿 Y 上移 10
    collection.model_matrix = DMat4::from_translation(DVec3::new(0.0, -10.0, 0.0));

    // 世界 y=-5 的点 → 局部 y = 5 → 内部
    assert!(!collection.is_clipped(DVec3::new(0.0, -5.0, 0.0)));
    // 世界 y=-15 的点 → 局部 y = -5 → 被裁剪
    assert!(collection.is_clipped(DVec3::new(0.0, -15.0, 0.0)));
}

#[test]
fn collection_model_matrix_default_identity() {
    let collection = ClippingPlaneCollection::new();
    assert_eq!(collection.model_matrix, DMat4::IDENTITY);
}

// ============================================================================
// ClippingPlaneCollection: 带模型矩阵的 isEmpty
// ============================================================================

#[test]
fn collection_empty_never_clips() {
    let collection = ClippingPlaneCollection::new();
    assert!(!collection.is_clipped(DVec3::new(-100.0, -100.0, -100.0)));
}

#[test]
fn collection_disabled_never_clips_even_with_planes() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::Y, 0.0),
    ]);
    collection.enabled = false;

    assert!(!collection.is_clipped(DVec3::new(0.0, -100.0, 0.0)));
}

// ============================================================================
// ClippingPlaneCollection: intersect_bounding_sphere 边缘情形
// ============================================================================

#[test]
fn intersect_bounding_sphere_empty_collection() {
    let collection = ClippingPlaneCollection::new();
    let result = collection.intersect_bounding_sphere(DVec3::ZERO, 1.0);
    assert_eq!(result, Intersect::Inside);
}

#[test]
fn intersect_bounding_sphere_disabled() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::Y, 0.0),
    ]);
    collection.enabled = false;

    let result = collection.intersect_bounding_sphere(DVec3::new(0.0, -10.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Inside);
}

#[test]
fn intersect_bounding_sphere_union_mode() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::Y, 0.0), // y >= 0
        ClippingPlane::new(DVec3::X, 0.0), // x >= 0
    ]);
    collection.union_clipping_regions = true;

    // 同时在两个内部 → inside
    let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 10.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Inside);

    // 在一个之外 → Outside（并集模式：任一平面裁剪则球体被裁剪）
    let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 10.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Outside);

    // 与一个相交 → intersecting
    let result = collection.intersect_bounding_sphere(DVec3::new(-0.5, 10.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Intersecting);
}

#[test]
fn intersect_bounding_sphere_intersection_mode_multi_plane() {
    let collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::Y, 0.0),
        ClippingPlane::new(DVec3::X, 0.0),
    ]);

    // 同时在两个内部 → inside
    let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 10.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Inside);

    // 在一个内部、另一个之外 → Inside（相交模式：球体被保留，
    // 因为每个点都在平面 Y 内部，所以没有点位于所有平面之外）
    let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 10.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Inside);

    // 同时在两个之外 → outside
    let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, -10.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Outside);
}

// ============================================================================
// ClippingPlaneCollection: edge_width
// ============================================================================

#[test]
fn collection_edge_width_default_zero() {
    let collection = ClippingPlaneCollection::new();
    assert!((collection.edge_width).abs() < EPSILON10);
}

#[test]
fn collection_edge_color_default_white() {
    let collection = ClippingPlaneCollection::new();
    assert!((collection.edge_color[0] - 1.0).abs() < EPSILON10);
    assert!((collection.edge_color[1] - 1.0).abs() < EPSILON10);
    assert!((collection.edge_color[2] - 1.0).abs() < EPSILON10);
    assert!((collection.edge_color[3] - 1.0).abs() < EPSILON10);
}

// ============================================================================
// ClippingPlane: 带偏移的 signed_distance
// ============================================================================

#[test]
fn plane_signed_distance_diagonal_normal() {
    let normal = DVec3::new(1.0, 1.0, 0.0).normalize();
    let plane = ClippingPlane::new(normal, 0.0);

    // 沿法线的点：正距离
    let sd = plane.signed_distance(normal * 3.0);
    assert!((sd - 3.0).abs() < EPSILON10);
}

#[test]
fn plane_is_inside_boundary() {
    let plane = ClippingPlane::new(DVec3::Y, -10.0);
    assert!(plane.is_inside(DVec3::new(0.0, 10.0, 0.0))); // y=10 → dot=10 + (-10) = 0 → 在平面上
    assert!(plane.is_inside(DVec3::new(0.0, 15.0, 0.0))); // y=15 → dot=15 + (-10) = 5 → 内部
    assert!(!plane.is_inside(DVec3::new(0.0, 5.0, 0.0))); // y=5 → dot=5 + (-10) = -5 → 外部
}

// ============================================================================
// ClippingPlane: transform 恒等往返
// ============================================================================

#[test]
fn plane_transform_identity_unchanged() {
    let plane = ClippingPlane::new(DVec3::new(1.0, 2.0, 3.0).normalize(), 7.5);
    let transformed = plane.transform(&DMat4::IDENTITY);

    assert!((transformed.normal - plane.normal).length() < EPSILON10);
    assert!((transformed.distance - plane.distance).abs() < EPSILON10);
}

// ============================================================================
// ClippingPlaneCollection: pack_planes 大型集合
// ============================================================================

#[test]
fn pack_planes_multiple() {
    let mut collection = ClippingPlaneCollection::new();
    collection.add(ClippingPlane::new(DVec3::X, 0.0));
    collection.add(ClippingPlane::new(DVec3::Y, 1.0));
    collection.add(ClippingPlane::new(DVec3::Z, 2.0));

    let packed = collection.pack_planes();
    assert_eq!(packed.len(), 12); // 3 planes * 4 values
    // 平面 0：X 法线，距离 0
    assert!((packed[0] - 1.0).abs() < EPSILON10, "p[0]: {}", packed[0]);
    assert!(packed[1].abs() < EPSILON10, "p[1]: {}", packed[1]);
    assert!(packed[2].abs() < EPSILON10, "p[2]: {}", packed[2]);
    assert!(packed[3].abs() < EPSILON10, "p[3]: {}", packed[3]);
    // 平面 1：Y 法线，距离 1
    assert!(packed[4].abs() < EPSILON10, "p[4]: {}", packed[4]);
    assert!((packed[5] - 1.0).abs() < EPSILON10, "p[5]: {}", packed[5]);
    assert!(packed[6].abs() < EPSILON10, "p[6]: {}", packed[6]);
    assert!((packed[7] - 1.0).abs() < EPSILON10, "p[7]: {}", packed[7]);
    // 平面 2：Z 法线，距离 2
    assert!(packed[8].abs() < EPSILON10, "p[8]: {}", packed[8]);
    assert!(packed[9].abs() < EPSILON10, "p[9]: {}", packed[9]);
    assert!((packed[10] - 1.0).abs() < EPSILON10, "p[10]: {}", packed[10]);
    assert!((packed[11] - 2.0).abs() < EPSILON10, "p[11]: {}", packed[11]);
}

// ============================================================================
// ClippingPlaneCollection: length 追踪
// ============================================================================

#[test]
fn collection_length_tracks_add_remove() {
    let mut collection = ClippingPlaneCollection::new();
    assert_eq!(collection.len(), 0);

    collection.add(ClippingPlane::new(DVec3::X, 0.0));
    assert_eq!(collection.len(), 1);

    collection.add(ClippingPlane::new(DVec3::Y, 0.0));
    assert_eq!(collection.len(), 2);

    collection.remove(0);
    assert_eq!(collection.len(), 1);

    collection.remove_all();
    assert_eq!(collection.len(), 0);
}
