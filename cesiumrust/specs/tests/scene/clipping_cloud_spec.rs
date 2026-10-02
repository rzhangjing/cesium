//! 裁剪与云规范 - 参考自 Scene/ClippingPlaneSpec, ClippingPlaneCollectionSpec, CloudCollectionSpec
//! 覆盖：ClippingPlane, ClippingPlaneCollection, CloudCollection, CumulusCloud, CloudType
//! ClippingPlaneSpec：3 个 A 类（共 5；1 callback=C, 1 result-param=C）
//! ClippingPlaneCollectionSpec：7 个 A 类（共 28；3 events=C, 18 WebGL=C）

use cesium_effects::{ClippingPlane, ClippingPlaneCollection, CloudCollection, CloudType};
use glam::{DMat4, DVec3};

// ─── ClippingPlane ──────────────────────────────────────────────────────────

#[test]
fn clipping_plane_creation() {
    let plane = ClippingPlane::new(DVec3::new(0.0, 0.0, 1.0), 5.0);
    assert_eq!(plane.normal, DVec3::new(0.0, 0.0, 1.0));
    assert_eq!(plane.distance, 5.0);
}

#[test]
fn clipping_plane_negative_distance() {
    let plane = ClippingPlane::new(DVec3::X, -10.0);
    assert_eq!(plane.distance, -10.0);
}

// ─── ClippingPlaneCollection ────────────────────────────────────────────────

#[test]
fn clipping_plane_collection_default() {
    let collection = ClippingPlaneCollection::default();
    assert!(collection.enabled);
    assert!(!collection.union_clipping_regions);
}

#[test]
fn clipping_plane_collection_add() {
    let mut collection = ClippingPlaneCollection::default();
    collection.add(ClippingPlane::new(DVec3::Z, 0.0));
    collection.add(ClippingPlane::new(DVec3::new(0.0, 0.0, -1.0), 0.0));
    assert_eq!(collection.len(), 2);
}

#[test]
fn clipping_plane_collection_union_mode() {
    let mut collection = ClippingPlaneCollection::default();
    collection.union_clipping_regions = true;
    assert!(collection.union_clipping_regions);
}

// ─── CloudCollection ────────────────────────────────────────────────────────

#[test]
fn cloud_collection_default() {
    let clouds = CloudCollection::default();
    assert_eq!(clouds.len(), 0);
}

#[test]
fn cloud_type_default() {
    assert_eq!(CloudType::default(), CloudType::Cumulus);
}

// ─── ClippingPlane：对齐实现自 ClippingPlaneSpec ──────────────────

#[test]
fn clipping_plane_constructs() {
    // 参考自: ClippingPlaneSpec "constructs"
    let normal = DVec3::X;
    let distance = 1.0;
    let plane = ClippingPlane::new(normal, distance);
    assert_eq!(plane.normal, normal);
    assert_eq!(plane.distance, distance);
}

#[test]
fn clipping_plane_works_with_plane_math() {
    // 参考自: ClippingPlaneSpec "works with Plane math"
    let normal = DVec3::new(1.0, 2.0, 3.0).normalize();
    let clipping_plane = ClippingPlane::new(normal, 12.34);

    // transform = fromUniformScale(2) * multiplyByMatrix3(fromRotationY(PI))
    // R_y(PI) = diag(-1, 1, -1), S(2) = diag(2, 2, 2)
    // Combined upper-left 3x3 = diag(-2, 2, -2)
    let transform = DMat4::from_cols_array(&[
        -2.0, 0.0, 0.0, 0.0,
        0.0, 2.0, 0.0, 0.0,
        0.0, 0.0, -2.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);

    let transformed = clipping_plane.transform(&transform);

    // CesiumJS 预期：distance * 2.0
    assert!(
        (transformed.distance - clipping_plane.distance * 2.0).abs() < 1e-10,
        "distance: got {}, expected {}",
        transformed.distance,
        clipping_plane.distance * 2.0
    );
    // normal.x 取反
    assert!(
        (transformed.normal.x - (-clipping_plane.normal.x)).abs() < 1e-10,
        "normal.x: got {}, expected {}",
        transformed.normal.x,
        -clipping_plane.normal.x
    );
    // normal.y 保持不变
    assert!(
        (transformed.normal.y - clipping_plane.normal.y).abs() < 1e-10,
        "normal.y"
    );
    // normal.z 取反
    assert!(
        (transformed.normal.z - (-clipping_plane.normal.z)).abs() < 1e-10,
        "normal.z"
    );
}

// ─── ClippingPlaneCollection：对齐实现自 ClippingPlaneCollectionSpec ───

#[test]
fn clipping_collection_default_constructor() {
    // 参考自: ClippingPlaneCollectionSpec "default constructor"
    let collection = ClippingPlaneCollection::default();
    assert!(collection.is_empty());
    assert!(collection.enabled);
    assert_eq!(collection.model_matrix, DMat4::IDENTITY);
    assert_eq!(collection.edge_color, [1.0, 1.0, 1.0, 1.0]); // WHITE
    assert_eq!(collection.edge_width, 0.0);
    assert!(!collection.union_clipping_regions);
}

#[test]
fn clipping_collection_get_at_index() {
    // 参考自: ClippingPlaneCollectionSpec "gets the plane at an index"
    let planes = vec![
        ClippingPlane::new(DVec3::X, 1.0),
        ClippingPlane::new(DVec3::Y, 2.0),
    ];
    let collection = ClippingPlaneCollection::with_planes(planes);

    let p0 = collection.get(0).unwrap();
    assert_eq!(p0.normal, DVec3::X);
    assert_eq!(p0.distance, 1.0);

    let p1 = collection.get(1).unwrap();
    assert_eq!(p1.normal, DVec3::Y);
    assert_eq!(p1.distance, 2.0);

    assert!(collection.get(2).is_none());
}

#[test]
fn clipping_collection_remove_first_occurrence() {
    // 参考自: ClippingPlaneCollectionSpec "remove removes the first occurrence"
    let planes = vec![
        ClippingPlane::new(DVec3::X, 1.0),
        ClippingPlane::new(DVec3::Y, 2.0),
    ];
    let mut collection = ClippingPlaneCollection::with_planes(planes);

    assert_eq!(collection.len(), 2);
    let removed = collection.remove(0);
    assert!(removed.is_some());
    assert_eq!(collection.len(), 1);

    // 剩余的平面应为 Y 平面
    let remaining = collection.get(0).unwrap();
    assert_eq!(remaining.normal, DVec3::Y);

    // 越界返回 None
    assert!(collection.remove(5).is_none());
}

#[test]
fn clipping_collection_remove_all() {
    // 参考自: ClippingPlaneCollectionSpec "removeAll removes all"
    let planes = vec![
        ClippingPlane::new(DVec3::X, 1.0),
        ClippingPlane::new(DVec3::Y, 2.0),
    ];
    let mut collection = ClippingPlaneCollection::with_planes(planes);
    assert_eq!(collection.len(), 2);

    collection.remove_all();
    assert_eq!(collection.len(), 0);
}

#[test]
fn clipping_collection_clipping_planes_state() {
    // 参考自: ClippingPlaneCollectionSpec behavior
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::X, 1.0),
        ClippingPlane::new(DVec3::Y, 2.0),
    ]);

    // 交集模式（默认）：负数计数
    assert_eq!(collection.clipping_planes_state(), -2);

    // 并集模式：正数计数
    collection.union_clipping_regions = true;
    assert_eq!(collection.clipping_planes_state(), 2);
}
