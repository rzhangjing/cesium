//! Tests for `cesium_core::EllipsoidTangentPlane`.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoid_tangent_plane::EllipsoidTangentPlane;

const EPSILON6: f64 = 1e-6;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

#[test]
fn constructor_returns_tangent_plane_on_surface() {
    // Point on the equator of the WGS84 ellipsoid
    let origin = Cartesian3::new(6378137.0, 0.0, 0.0);
    let tp = EllipsoidTangentPlane::new(&origin, None);
    assert!(tp.is_some());
    let tp = tp.unwrap();
    // Origin should be on the ellipsoid surface
    assert!(approx_eq(tp.origin().x, 6378137.0, 1.0));
    assert!(approx_eq(tp.origin().y, 0.0, 1.0));
    assert!(approx_eq(tp.origin().z, 0.0, 1.0));
}

#[test]
fn constructor_with_point_above_surface_projects_to_surface() {
    let above = Cartesian3::new(6378137.0 * 2.0, 0.0, 0.0);
    let tp = EllipsoidTangentPlane::new(&above, None);
    assert!(tp.is_some());
    let tp = tp.unwrap();
    // The origin should be projected to the ellipsoid surface
    assert!(approx_eq(tp.origin().x, 6378137.0, 1.0));
}

#[test]
fn x_axis_is_east_direction() {
    let origin = Cartesian3::new(6378137.0, 0.0, 0.0);
    let tp = EllipsoidTangentPlane::new(&origin, None).unwrap();
    let x = tp.x_axis();
    // At the equator on the prime meridian, east is (0, 1, 0)
    assert!(approx_eq(x.x, 0.0, EPSILON6));
    assert!(approx_eq(x.y, 1.0, EPSILON6));
    assert!(approx_eq(x.z, 0.0, EPSILON6));
}

#[test]
fn y_axis_is_north_direction() {
    let origin = Cartesian3::new(6378137.0, 0.0, 0.0);
    let tp = EllipsoidTangentPlane::new(&origin, None).unwrap();
    let y = tp.y_axis();
    // At the equator on the prime meridian, north is (0, 0, 1)
    assert!(approx_eq(y.x, 0.0, EPSILON6));
    assert!(approx_eq(y.y, 0.0, EPSILON6));
    assert!(approx_eq(y.z, 1.0, EPSILON6));
}

#[test]
fn project_point_to_tangent_plane() {
    let origin = Cartesian3::new(6378137.0, 0.0, 0.0);
    let tp = EllipsoidTangentPlane::new(&origin, None).unwrap();

    // Project the origin itself => should be (0, 0)
    let result = tp.project_point_to_nearest_tangent_plane(&origin);
    assert!(approx_eq(result.x, 0.0, 1.0));
    assert!(approx_eq(result.y, 0.0, 1.0));
}

#[test]
fn project_point_offset_from_origin() {
    let origin = Cartesian3::new(6378137.0, 0.0, 0.0);
    let tp = EllipsoidTangentPlane::new(&origin, None).unwrap();

    // A point slightly east (in the y direction)
    let east_point = Cartesian3::new(6378137.0, 1000.0, 0.0);
    let result = tp.project_point_to_nearest_tangent_plane(&east_point);
    // x should be ~0 (along east axis), y should be small
    assert!(approx_eq(result.x, 1000.0, 10.0));
}

#[test]
fn ellipsoid_accessor() {
    let origin = Cartesian3::new(6378137.0, 0.0, 0.0);
    let tp = EllipsoidTangentPlane::new(&origin, Some(Ellipsoid::WGS84)).unwrap();
    assert!(tp.ellipsoid() == &Ellipsoid::WGS84);
}
