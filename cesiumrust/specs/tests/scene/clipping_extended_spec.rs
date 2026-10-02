//! ClippingPlane + ClippingPlaneCollection 扩展 specs
//! 参考自 CesiumJS Scene/ClippingPlaneSpec + Scene/ClippingPlaneCollectionSpec

use cesium_effects::{ClippingPlane, ClippingPlaneCollection, Intersect};
use glam::{DMat4, DVec3};

// ==================== ClippingPlane ====================

#[test]
fn clipping_plane_new_normalizes() {
    let plane = ClippingPlane::new(DVec3::new(2.0, 0.0, 0.0), 5.0);
    assert!((plane.normal.length() - 1.0).abs() < 1e-10);
    assert!((plane.normal.x - 1.0).abs() < 1e-10);
    assert!((plane.distance - 5.0).abs() < 1e-10);
}

#[test]
fn clipping_plane_signed_distance() {
    let plane = ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0);
    // 位于正侧的点
    assert!((plane.signed_distance(DVec3::new(5.0, 0.0, 0.0)) - 5.0).abs() < 1e-10);
    // 位于负侧的点
    assert!((plane.signed_distance(DVec3::new(-3.0, 0.0, 0.0)) - (-3.0)).abs() < 1e-10);
    // 位于平面上的点
    assert!((plane.signed_distance(DVec3::new(0.0, 5.0, 3.0))).abs() < 1e-10);
}

#[test]
fn clipping_plane_is_inside() {
    let plane = ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), -2.0);
    // 内部：dot(normal, point) + distance >= 0 → y - 2 >= 0 → y >= 2
    assert!(plane.is_inside(DVec3::new(0.0, 3.0, 0.0)));
    assert!(plane.is_inside(DVec3::new(0.0, 2.0, 0.0))); // 在平面上
    assert!(!plane.is_inside(DVec3::new(0.0, 1.0, 0.0)));
}

#[test]
fn clipping_plane_to_from_vec4() {
    let plane = ClippingPlane::new(DVec3::new(0.0, 0.0, 1.0), -10.0);
    let packed = plane.to_vec4();
    let unpacked = ClippingPlane::from_vec4(packed);
    assert!((unpacked.normal - plane.normal).length() < 1e-10);
    assert!((unpacked.distance - plane.distance).abs() < 1e-10);
}

#[test]
fn clipping_plane_transform_translation() {
    let plane = ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0);
    // 平移 (5, 0, 0)
    let matrix = DMat4::from_translation(DVec3::new(5.0, 0.0, 0.0));
    let transformed = plane.transform(&matrix);
    // 法线应保持不变（平移不影响法线）
    assert!((transformed.normal.x - 1.0).abs() < 1e-10);
    // 距离应改变：位于 x=0 的平面移动到 x=5 → distance = -5
    assert!((transformed.distance - (-5.0)).abs() < 1e-10);
}

// ==================== ClippingPlaneCollection ====================

#[test]
fn collection_default_state() {
    let collection = ClippingPlaneCollection::new();
    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);
    assert!(collection.enabled);
    assert!(!collection.union_clipping_regions);
    assert!((collection.edge_width).abs() < 1e-10);
}

#[test]
fn collection_add_and_get() {
    let mut collection = ClippingPlaneCollection::new();
    collection.add(ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0));
    collection.add(ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), -5.0));
    assert_eq!(collection.len(), 2);
    assert!(!collection.is_empty());

    let p = collection.get(1).unwrap();
    assert!((p.normal.y - 1.0).abs() < 1e-10);
    assert!((p.distance - (-5.0)).abs() < 1e-10);
}

#[test]
fn collection_remove() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0),
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 0.0),
    ]);
    let removed = collection.remove(0);
    assert!(removed.is_some());
    assert_eq!(collection.len(), 1);
    // 越界
    assert!(collection.remove(5).is_none());
}

#[test]
fn collection_remove_all() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0),
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 0.0),
    ]);
    collection.remove_all();
    assert!(collection.is_empty());
}

#[test]
fn collection_clipping_planes_state() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0),
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 0.0),
        ClippingPlane::new(DVec3::new(0.0, 0.0, 1.0), 0.0),
    ]);
    // 相交模式（默认）：结果为负
    assert_eq!(collection.clipping_planes_state(), -3);
    // 并集模式：结果为正
    collection.union_clipping_regions = true;
    assert_eq!(collection.clipping_planes_state(), 3);
}

#[test]
fn collection_is_clipped_intersection_mode() {
    // 相交模式：仅当位于所有平面之外时才裁剪
    let collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0), // x >= 0
        ClippingPlane::new(DVec3::new(-1.0, 0.0, 0.0), 10.0), // x <= 10
    ]);
    // 同时在两个内部 → 不裁剪
    assert!(!collection.is_clipped(DVec3::new(5.0, 0.0, 0.0)));
    // 在一个之外但在另一个之内 → 不裁剪（相交模式）
    assert!(!collection.is_clipped(DVec3::new(-1.0, 0.0, 0.0)));
    // Outside both (x>10: outside plane1's x>=0? No. Let's use x=-5: outside x>=0 AND outside x<=10)
    // x=-5: plane1 signed_dist=-5<0(outside), plane2 signed_dist=5+10=15>0(inside)
    // Need outside BOTH: x=15 → plane1: 15>0(inside), plane2: -15+10=-5<0(outside)
    // 实际上对于相交模式，我们需要一个位于所有平面之外的点。
    // Plane1 保留 x>=0，Plane2 保留 x<=10。同时在两者之外 = 对有限 x 不可能。
    // 改用另一种设置：两个平面构成一个角
    let collection2 = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0), // x >= 0
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 0.0), // y >= 0
    ]);
    // 同时在两者之外：x<0 且 y<0
    assert!(collection2.is_clipped(DVec3::new(-1.0, -1.0, 0.0)));
    // 仅在一个之外 → 相交模式下不裁剪
    assert!(!collection2.is_clipped(DVec3::new(-1.0, 5.0, 0.0)));
}

#[test]
fn collection_is_clipped_union_mode() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0), // x >= 0
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 0.0), // y >= 0
    ]);
    collection.union_clipping_regions = true;
    // 同时在两个内部 → 不裁剪
    assert!(!collection.is_clipped(DVec3::new(1.0, 1.0, 0.0)));
    // 位于任一之外 → 裁剪（并集模式）
    assert!(collection.is_clipped(DVec3::new(-1.0, 1.0, 0.0)));
    assert!(collection.is_clipped(DVec3::new(1.0, -1.0, 0.0)));
}

#[test]
fn collection_disabled_never_clips() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
        DVec3::new(1.0, 0.0, 0.0),
        0.0,
    )]);
    collection.enabled = false;
    assert!(!collection.is_clipped(DVec3::new(-100.0, 0.0, 0.0)));
}

#[test]
fn collection_intersect_bounding_sphere_inside() {
    let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
        DVec3::new(1.0, 0.0, 0.0),
        0.0,
    )]);
    // 球体完全位于正侧
    let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 0.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Inside);
}

#[test]
fn collection_intersect_bounding_sphere_outside() {
    let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
        DVec3::new(1.0, 0.0, 0.0),
        0.0,
    )]);
    // 球体完全位于负侧
    let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 0.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Outside);
}

#[test]
fn collection_intersect_bounding_sphere_intersecting() {
    let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
        DVec3::new(1.0, 0.0, 0.0),
        0.0,
    )]);
    // 球体横跨平面
    let result = collection.intersect_bounding_sphere(DVec3::new(0.5, 0.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Intersecting);
}

#[test]
fn collection_pack_planes() {
    let collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 5.0),
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), -3.0),
    ]);
    let packed = collection.pack_planes();
    assert_eq!(packed.len(), 8); // 2 planes * 4 values
    assert!((packed[0] - 1.0).abs() < 1e-10); // 第一个平面 normal.x
    assert!((packed[3] - 5.0).abs() < 1e-10); // 第一个平面 distance
    assert!((packed[5] - 1.0).abs() < 1e-10); // 第二个平面 normal.y
    assert!((packed[7] - (-3.0)).abs() < 1e-10); // 第二个平面 distance
}
