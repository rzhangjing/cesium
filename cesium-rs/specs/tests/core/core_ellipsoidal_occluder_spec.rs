//! Tests for `cesium_core::EllipsoidalOccluder`.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoidal_occluder::EllipsoidalOccluder;

#[test]
fn constructor_sets_ellipsoid() {
    let occluder = EllipsoidalOccluder::new(Ellipsoid::WGS84, None);
    assert!(occluder.ellipsoid() == &Ellipsoid::WGS84);
}

#[test]
fn constructor_with_camera_position() {
    let camera = Cartesian3::new(0.0, 0.0, 10e6);
    let occluder = EllipsoidalOccluder::new(Ellipsoid::WGS84, Some(&camera));
    assert!((occluder.camera_position().z - 10e6).abs() < 1e-6);
}

#[test]
fn point_on_same_side_as_camera_is_visible() {
    let camera = Cartesian3::new(0.0, 0.0, 10e6);
    let mut occluder = EllipsoidalOccluder::new(Ellipsoid::WGS84, None);
    occluder.set_camera_position(&camera);

    // Point near the camera (above the north pole)
    let point = Cartesian3::new(0.0, 0.0, 6378137.0 + 100.0);
    assert!(occluder.is_point_visible(&point));
}

#[test]
fn point_on_opposite_side_is_not_visible() {
    let camera = Cartesian3::new(0.0, 0.0, 10e6);
    let mut occluder = EllipsoidalOccluder::new(Ellipsoid::WGS84, None);
    occluder.set_camera_position(&camera);

    // Point on the opposite side of the ellipsoid
    let point = Cartesian3::new(0.0, 0.0, -(6378137.0 + 100.0));
    assert!(!occluder.is_point_visible(&point));
}

#[test]
fn bounding_sphere_visible_when_center_visible() {
    let camera = Cartesian3::new(0.0, 0.0, 10e6);
    let mut occluder = EllipsoidalOccluder::new(Ellipsoid::WGS84, None);
    occluder.set_camera_position(&camera);

    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 6378137.0 + 100.0), 10.0);
    assert!(occluder.is_bounding_sphere_visible(&bs));
}

#[test]
fn bounding_sphere_not_visible_when_behind_ellipsoid() {
    let camera = Cartesian3::new(0.0, 0.0, 10e6);
    let mut occluder = EllipsoidalOccluder::new(Ellipsoid::WGS84, None);
    occluder.set_camera_position(&camera);

    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, -(6378137.0 + 100.0)), 10.0);
    assert!(!occluder.is_bounding_sphere_visible(&bs));
}
