//! ClippingPlane + ClippingPlaneCollection extended specs
//! Ported from CesiumJS Scene/ClippingPlaneSpec.js + Scene/ClippingPlaneCollectionSpec.js

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
    // Point on positive side
    assert!((plane.signed_distance(DVec3::new(5.0, 0.0, 0.0)) - 5.0).abs() < 1e-10);
    // Point on negative side
    assert!((plane.signed_distance(DVec3::new(-3.0, 0.0, 0.0)) - (-3.0)).abs() < 1e-10);
    // Point on plane
    assert!((plane.signed_distance(DVec3::new(0.0, 5.0, 3.0))).abs() < 1e-10);
}

#[test]
fn clipping_plane_is_inside() {
    let plane = ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), -2.0);
    // Inside: dot(normal, point) + distance >= 0 → y - 2 >= 0 → y >= 2
    assert!(plane.is_inside(DVec3::new(0.0, 3.0, 0.0)));
    assert!(plane.is_inside(DVec3::new(0.0, 2.0, 0.0))); // On plane
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
    // Translate by (5, 0, 0)
    let matrix = DMat4::from_translation(DVec3::new(5.0, 0.0, 0.0));
    let transformed = plane.transform(&matrix);
    // Normal should remain the same (translation doesn't affect normals)
    assert!((transformed.normal.x - 1.0).abs() < 1e-10);
    // Distance should change: plane at x=0 moved to x=5 → distance = -5
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
    // Out of bounds
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
    // Intersection mode (default): negative
    assert_eq!(collection.clipping_planes_state(), -3);
    // Union mode: positive
    collection.union_clipping_regions = true;
    assert_eq!(collection.clipping_planes_state(), 3);
}

#[test]
fn collection_is_clipped_intersection_mode() {
    // Intersection mode: clip only if outside ALL planes
    let collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0), // x >= 0
        ClippingPlane::new(DVec3::new(-1.0, 0.0, 0.0), 10.0), // x <= 10
    ]);
    // Inside both → not clipped
    assert!(!collection.is_clipped(DVec3::new(5.0, 0.0, 0.0)));
    // Outside one but inside other → not clipped (intersection mode)
    assert!(!collection.is_clipped(DVec3::new(-1.0, 0.0, 0.0)));
    // Outside both (x>10: outside plane1's x>=0? No. Let's use x=-5: outside x>=0 AND outside x<=10)
    // x=-5: plane1 signed_dist=-5<0(outside), plane2 signed_dist=5+10=15>0(inside)
    // Need outside BOTH: x=15 → plane1: 15>0(inside), plane2: -15+10=-5<0(outside)
    // Actually for intersection mode, we need a point outside ALL planes.
    // Plane1 keeps x>=0, Plane2 keeps x<=10. Outside both = impossible for finite x.
    // Use a different setup: two planes forming a corner
    let collection2 = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0), // x >= 0
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 0.0), // y >= 0
    ]);
    // Outside both: x<0 AND y<0
    assert!(collection2.is_clipped(DVec3::new(-1.0, -1.0, 0.0)));
    // Outside one only → not clipped in intersection mode
    assert!(!collection2.is_clipped(DVec3::new(-1.0, 5.0, 0.0)));
}

#[test]
fn collection_is_clipped_union_mode() {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        ClippingPlane::new(DVec3::new(1.0, 0.0, 0.0), 0.0), // x >= 0
        ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 0.0), // y >= 0
    ]);
    collection.union_clipping_regions = true;
    // Inside both → not clipped
    assert!(!collection.is_clipped(DVec3::new(1.0, 1.0, 0.0)));
    // Outside ANY → clipped (union mode)
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
    // Sphere fully on positive side
    let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 0.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Inside);
}

#[test]
fn collection_intersect_bounding_sphere_outside() {
    let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
        DVec3::new(1.0, 0.0, 0.0),
        0.0,
    )]);
    // Sphere fully on negative side
    let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 0.0, 0.0), 1.0);
    assert_eq!(result, Intersect::Outside);
}

#[test]
fn collection_intersect_bounding_sphere_intersecting() {
    let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
        DVec3::new(1.0, 0.0, 0.0),
        0.0,
    )]);
    // Sphere straddles the plane
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
    assert!((packed[0] - 1.0).abs() < 1e-10); // First plane normal.x
    assert!((packed[3] - 5.0).abs() < 1e-10); // First plane distance
    assert!((packed[5] - 1.0).abs() < 1e-10); // Second plane normal.y
    assert!((packed[7] - (-3.0)).abs() < 1e-10); // Second plane distance
}
