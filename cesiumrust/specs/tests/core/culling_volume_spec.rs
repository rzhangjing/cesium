//! Ported from CullingVolumeSpec.js (43 it(), 40 A-class)
//!
//! 3 throws = C-class (Rust type system enforces valid inputs).
//! Each A-class test verifies both computeVisibility and computeVisibilityWithPlaneMask.

use cesium_geospatial::bounding::{AxisAlignedBoundingBox, BoundingSphere};
use cesium_geospatial::frustum::{Cullable, CullingVolume, PerspectiveFrustum};
use cesium_geospatial::ray::Intersect;
use glam::DVec3;
use std::f64::consts::PI;

/// Creates the standard test culling volume: perspective frustum fov=PI/3, aspect=1, near=1, far=2,
/// positioned at origin looking down -Z with up=+Y.
fn create_culling_volume() -> CullingVolume {
    let frustum = PerspectiveFrustum::new(PI / 3.0, 1.0, 1.0, 2.0);
    frustum.compute_culling_volume(DVec3::ZERO, -DVec3::Z, DVec3::Y)
}

/// Mirrors the CesiumJS test helper `testWithAndWithoutPlaneMask`.
fn assert_visibility(cv: &CullingVolume, volume: &impl Cullable, expected: Intersect) {
    // Test computeVisibility
    assert_eq!(cv.visibility(volume), expected);

    // Test computeVisibilityWithPlaneMask
    let mask = cv.visibility_with_plane_mask(volume, CullingVolume::MASK_INDETERMINATE);
    match expected {
        Intersect::Inside => assert_eq!(mask, CullingVolume::MASK_INSIDE),
        Intersect::Outside => assert_eq!(mask, CullingVolume::MASK_OUTSIDE),
        Intersect::Intersecting => {
            assert_ne!(mask, CullingVolume::MASK_INSIDE);
            assert_ne!(mask, CullingVolume::MASK_OUTSIDE);
        }
    }
    // Idempotency: applying the mask again returns the same mask
    assert_eq!(cv.visibility_with_plane_mask(volume, mask), mask);
}

// ===== Box intersections =====

#[test]
fn culling_box_inside() {
    let cv = create_culling_volume();
    let box1 = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, 0.0, -1.25),
        DVec3::new(0.5, 0.0, -1.25),
        DVec3::new(-0.5, 0.0, -1.75),
        DVec3::new(0.5, 0.0, -1.75),
    ]);
    assert_visibility(&cv, &box1, Intersect::Inside);
}

#[test]
fn culling_box_intersect_far() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, 0.0, -1.5),
        DVec3::new(0.5, 0.0, -1.5),
        DVec3::new(-0.5, 0.0, -2.5),
        DVec3::new(0.5, 0.0, -2.5),
    ]);
    assert_visibility(&cv, &b, Intersect::Intersecting);
}

#[test]
fn culling_box_intersect_near() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, 0.0, -0.5),
        DVec3::new(0.5, 0.0, -0.5),
        DVec3::new(-0.5, 0.0, -1.5),
        DVec3::new(0.5, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &b, Intersect::Intersecting);
}

#[test]
fn culling_box_intersect_left() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-1.5, 0.0, -1.25),
        DVec3::new(0.0, 0.0, -1.25),
        DVec3::new(-1.5, 0.0, -1.5),
        DVec3::new(0.0, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &b, Intersect::Intersecting);
}

#[test]
fn culling_box_intersect_right() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(0.0, 0.0, -1.25),
        DVec3::new(1.5, 0.0, -1.25),
        DVec3::new(0.0, 0.0, -1.5),
        DVec3::new(1.5, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &b, Intersect::Intersecting);
}

#[test]
fn culling_box_intersect_top() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, 0.0, -1.25),
        DVec3::new(0.5, 0.0, -1.25),
        DVec3::new(-0.5, 2.0, -1.75),
        DVec3::new(0.5, 2.0, -1.75),
    ]);
    assert_visibility(&cv, &b, Intersect::Intersecting);
}

#[test]
fn culling_box_intersect_bottom() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, -2.0, -1.25),
        DVec3::new(0.5, 0.0, -1.25),
        DVec3::new(-0.5, -2.0, -1.5),
        DVec3::new(0.5, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &b, Intersect::Intersecting);
}

#[test]
fn culling_box_outside_far() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, 0.0, -2.25),
        DVec3::new(0.5, 0.0, -2.25),
        DVec3::new(-0.5, 0.0, -2.75),
        DVec3::new(0.5, 0.0, -2.75),
    ]);
    assert_visibility(&cv, &b, Intersect::Outside);
}

#[test]
fn culling_box_outside_near() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, 0.0, -0.25),
        DVec3::new(0.5, 0.0, -0.25),
        DVec3::new(-0.5, 0.0, -0.75),
        DVec3::new(0.5, 0.0, -0.75),
    ]);
    assert_visibility(&cv, &b, Intersect::Outside);
}

#[test]
fn culling_box_outside_left() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-5.0, 0.0, -1.25),
        DVec3::new(-3.0, 0.0, -1.25),
        DVec3::new(-5.0, 0.0, -1.75),
        DVec3::new(-3.0, 0.0, -1.75),
    ]);
    assert_visibility(&cv, &b, Intersect::Outside);
}

#[test]
fn culling_box_outside_right() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(3.0, 0.0, -1.25),
        DVec3::new(5.0, 0.0, -1.25),
        DVec3::new(3.0, 0.0, -1.75),
        DVec3::new(5.0, 0.0, -1.75),
    ]);
    assert_visibility(&cv, &b, Intersect::Outside);
}

#[test]
fn culling_box_outside_top() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, 3.0, -1.25),
        DVec3::new(0.5, 3.0, -1.25),
        DVec3::new(-0.5, 5.0, -1.75),
        DVec3::new(0.5, 5.0, -1.75),
    ]);
    assert_visibility(&cv, &b, Intersect::Outside);
}

#[test]
fn culling_box_outside_bottom() {
    let cv = create_culling_volume();
    let b = AxisAlignedBoundingBox::from_points(&[
        DVec3::new(-0.5, -3.0, -1.25),
        DVec3::new(0.5, -3.0, -1.25),
        DVec3::new(-0.5, -5.0, -1.75),
        DVec3::new(0.5, -5.0, -1.75),
    ]);
    assert_visibility(&cv, &b, Intersect::Outside);
}

// ===== Sphere intersections =====

#[test]
fn culling_sphere_inside() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, 0.0, -1.25),
        DVec3::new(0.0, 0.0, -1.75),
    ]);
    assert_visibility(&cv, &s, Intersect::Inside);
}

#[test]
fn culling_sphere_intersect_far() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, 0.0, -1.5),
        DVec3::new(0.0, 0.0, -2.5),
    ]);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_sphere_intersect_near() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, 0.0, -0.5),
        DVec3::new(0.0, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_sphere_intersect_left() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(-1.0, 0.0, -1.5),
        DVec3::new(0.0, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_sphere_intersect_right() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, 0.0, -1.5),
        DVec3::new(1.0, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_sphere_intersect_top() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, 0.0, -1.5),
        DVec3::new(0.0, 2.0, -1.5),
    ]);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_sphere_intersect_bottom() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, -2.0, -1.5),
        DVec3::new(0.0, 0.0, -1.5),
    ]);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_sphere_outside_far() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, 0.0, -2.25),
        DVec3::new(0.0, 0.0, -2.75),
    ]);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_sphere_outside_near() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(0.0, 0.0, -0.25),
        DVec3::new(0.0, 0.0, -0.5),
    ]);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_sphere_outside_left() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(-5.0, 0.0, -1.25),
        DVec3::new(-4.5, 0.0, -1.75),
    ]);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_sphere_outside_right() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(4.5, 0.0, -1.25),
        DVec3::new(5.0, 0.0, -1.75),
    ]);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_sphere_outside_top() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(-0.5, 4.5, -1.25),
        DVec3::new(-0.5, 5.0, -1.25),
    ]);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_sphere_outside_bottom() {
    let cv = create_culling_volume();
    let s = BoundingSphere::from_points(&[
        DVec3::new(-0.5, -4.5, -1.25),
        DVec3::new(-0.5, -5.0, -1.25),
    ]);
    assert_visibility(&cv, &s, Intersect::Outside);
}

// ===== Construct from bounding sphere =====

const BS_CENTER: DVec3 = DVec3::new(1000.0, 2000.0, 3000.0);
const BS_RADIUS: f64 = 100.0;

fn from_sphere_culling_volume() -> CullingVolume {
    let sphere = BoundingSphere::new(BS_CENTER, BS_RADIUS);
    CullingVolume::from_bounding_sphere(&sphere)
}

#[test]
fn culling_from_sphere_inside() {
    let cv = from_sphere_culling_volume();
    let s = BoundingSphere::new(BS_CENTER, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Inside);
}

#[test]
fn culling_from_sphere_intersect_far() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, 0.0, BS_RADIUS * 1.5);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_from_sphere_intersect_near() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, 0.0, -BS_RADIUS * 1.5);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_from_sphere_intersect_left() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(-BS_RADIUS * 1.5, 0.0, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_from_sphere_intersect_right() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(BS_RADIUS * 1.5, 0.0, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_from_sphere_intersect_top() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, BS_RADIUS * 1.5, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_from_sphere_intersect_bottom() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, -BS_RADIUS * 1.5, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Intersecting);
}

#[test]
fn culling_from_sphere_outside_far() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, 0.0, BS_RADIUS * 2.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_from_sphere_outside_near() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, 0.0, -BS_RADIUS * 2.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_from_sphere_outside_left() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(-BS_RADIUS * 2.0, 0.0, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_from_sphere_outside_right() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(BS_RADIUS * 2.0, 0.0, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_from_sphere_outside_top() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, BS_RADIUS * 2.0, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Outside);
}

#[test]
fn culling_from_sphere_outside_bottom() {
    let cv = from_sphere_culling_volume();
    let center = BS_CENTER + DVec3::new(0.0, -BS_RADIUS * 2.0, 0.0);
    let s = BoundingSphere::new(center, BS_RADIUS * 0.5);
    assert_visibility(&cv, &s, Intersect::Outside);
}
