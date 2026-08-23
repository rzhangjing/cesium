//! Tests for `cesium_core::CullingVolume`.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::culling_volume::{CullingVolume, MASK_INSIDE, MASK_OUTSIDE};
use cesium_core::intersect::Intersect;

#[test]
fn from_bounding_sphere_creates_six_planes() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let cv = CullingVolume::from_bounding_sphere(&bs, None);
    assert_eq!(cv.planes.len(), 6);
}

#[test]
fn sphere_inside_culling_volume_is_inside() {
    // Create a culling volume from a large sphere
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 100.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    // A small sphere at the center should be inside
    let small_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility(&small_bs);
    assert_eq!(result, Intersect::Inside);
}

#[test]
fn sphere_outside_culling_volume_is_outside() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 10.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    // A sphere far away should be outside
    let far_bs = BoundingSphere::new(Cartesian3::new(1000.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility(&far_bs);
    assert_eq!(result, Intersect::Outside);
}

#[test]
fn sphere_intersecting_plane_is_intersecting() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 10.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    // A sphere near the edge should be intersecting
    let edge_bs = BoundingSphere::new(Cartesian3::new(9.5, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility(&edge_bs);
    assert_eq!(result, Intersect::Intersecting);
}

#[test]
fn compute_visibility_with_plane_mask_outside_returns_outside() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 100.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    let small_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility_with_plane_mask(&small_bs, MASK_OUTSIDE);
    assert_eq!(result, MASK_OUTSIDE);
}

#[test]
fn compute_visibility_with_plane_mask_inside_returns_inside() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 100.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    let small_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility_with_plane_mask(&small_bs, MASK_INSIDE);
    assert_eq!(result, MASK_INSIDE);
}

#[test]
fn default_has_no_planes() {
    let cv = CullingVolume::default();
    assert!(cv.planes.is_empty());
}
