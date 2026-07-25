//! Core/EllipsoidSpec.js → Rust integration tests
//! Tests for cesium_geospatial::ellipsoid::Ellipsoid

use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::math_utils::*;
use cesium_specs::{assert_approx, assert_vec3_epsilon, epsilon};
use glam::DVec3;

#[test]
fn test_ellipsoid_wgs84_radii() {
    let e = Ellipsoid::WGS84;
    let radii = e.radii();
    assert_approx!(radii.x, 6378137.0, epsilon::EPSILON1);
    assert_approx!(radii.y, 6378137.0, epsilon::EPSILON1);
    assert_approx!(radii.z, 6356752.3142451793, epsilon::EPSILON7);
}

#[test]
fn test_ellipsoid_unit_sphere() {
    let e = Ellipsoid::UNIT_SPHERE;
    let radii = e.radii();
    assert_approx!(radii.x, 1.0, epsilon::EPSILON15);
    assert_approx!(radii.y, 1.0, epsilon::EPSILON15);
    assert_approx!(radii.z, 1.0, epsilon::EPSILON15);
}

#[test]
fn test_ellipsoid_moon() {
    let e = Ellipsoid::MOON;
    let radii = e.radii();
    assert_approx!(radii.x, 1738100.0, epsilon::EPSILON1);
    assert_approx!(radii.y, 1738100.0, epsilon::EPSILON1);
    assert_approx!(radii.z, 1736000.0, epsilon::EPSILON1);
}

#[test]
fn test_ellipsoid_new() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let radii = e.radii();
    assert_approx!(radii.x, 1.0, epsilon::EPSILON15);
    assert_approx!(radii.y, 2.0, epsilon::EPSILON15);
    assert_approx!(radii.z, 3.0, epsilon::EPSILON15);
}

#[test]
#[should_panic]
fn test_ellipsoid_new_negative_radii_panics() {
    let _ = Ellipsoid::new(-1.0, 2.0, 3.0);
}

#[test]
fn test_ellipsoid_radii_squared() {
    let e = Ellipsoid::new(2.0, 3.0, 4.0);
    let rs = e.radii_squared();
    assert_approx!(rs.x, 4.0, epsilon::EPSILON15);
    assert_approx!(rs.y, 9.0, epsilon::EPSILON15);
    assert_approx!(rs.z, 16.0, epsilon::EPSILON15);
}

#[test]
fn test_ellipsoid_minimum_maximum_radius() {
    let e = Ellipsoid::new(3.0, 1.0, 2.0);
    assert_approx!(e.minimum_radius(), 1.0, epsilon::EPSILON15);
    assert_approx!(e.maximum_radius(), 3.0, epsilon::EPSILON15);
}

#[test]
fn test_ellipsoid_cartographic_to_cartesian() {
    let e = Ellipsoid::WGS84;
    let carto = Cartographic::from_radians(0.0, 0.0, 0.0);
    let cartesian = e.cartographic_to_cartesian(&carto);
    assert_approx!(cartesian.x, 6378137.0, epsilon::EPSILON1);
    assert_approx!(cartesian.y, 0.0, epsilon::EPSILON1);
    assert_approx!(cartesian.z, 0.0, epsilon::EPSILON1);
}

#[test]
fn test_ellipsoid_cartographic_to_cartesian_north_pole() {
    let e = Ellipsoid::WGS84;
    let carto = Cartographic::from_radians(0.0, PI_OVER_TWO, 0.0);
    let cartesian = e.cartographic_to_cartesian(&carto);
    assert_approx!(cartesian.x, 0.0, epsilon::EPSILON1);
    assert_approx!(cartesian.y, 0.0, epsilon::EPSILON1);
    assert_approx!(cartesian.z, 6356752.3142451793, epsilon::EPSILON5);
}

#[test]
fn test_ellipsoid_cartesian_to_cartographic() {
    let e = Ellipsoid::WGS84;
    let cartesian = DVec3::new(6378137.0, 0.0, 0.0);
    let carto = e.cartesian_to_cartographic(cartesian).unwrap();
    assert_approx!(carto.longitude, 0.0, epsilon::EPSILON10);
    assert_approx!(carto.latitude, 0.0, epsilon::EPSILON10);
    assert_approx!(carto.height, 0.0, epsilon::EPSILON5);
}

#[test]
fn test_ellipsoid_cartographic_roundtrip() {
    let e = Ellipsoid::WGS84;
    let original = Cartographic::from_radians(to_radians(45.0), to_radians(30.0), 1000.0);
    let cartesian = e.cartographic_to_cartesian(&original);
    let result = e.cartesian_to_cartographic(cartesian).unwrap();
    assert_approx!(result.longitude, original.longitude, epsilon::EPSILON10);
    assert_approx!(result.latitude, original.latitude, epsilon::EPSILON10);
    assert_approx!(result.height, original.height, epsilon::EPSILON5);
}

#[test]
fn test_ellipsoid_geodetic_surface_normal() {
    let e = Ellipsoid::WGS84;
    let point = DVec3::new(6378137.0, 0.0, 0.0);
    let normal = e.geodetic_surface_normal(point).unwrap();
    assert_vec3_epsilon!(normal, DVec3::new(1.0, 0.0, 0.0), epsilon::EPSILON10);
}

#[test]
fn test_ellipsoid_geodetic_surface_normal_north_pole() {
    let e = Ellipsoid::WGS84;
    let point = DVec3::new(0.0, 0.0, 6356752.3142451793);
    let normal = e.geodetic_surface_normal(point).unwrap();
    assert_vec3_epsilon!(normal, DVec3::new(0.0, 0.0, 1.0), epsilon::EPSILON10);
}

#[test]
fn test_ellipsoid_scale_to_geodetic_surface() {
    let e = Ellipsoid::WGS84;
    let point = DVec3::new(6378137.0 + 1000.0, 0.0, 0.0);
    let surface = e.scale_to_geodetic_surface(point).unwrap();
    assert_approx!(surface.x, 6378137.0, epsilon::EPSILON5);
    assert_approx!(surface.y, 0.0, epsilon::EPSILON5);
    assert_approx!(surface.z, 0.0, epsilon::EPSILON5);
}

#[test]
fn test_ellipsoid_one_over_radii() {
    let e = Ellipsoid::new(2.0, 4.0, 5.0);
    let oor = e.one_over_radii();
    assert_approx!(oor.x, 0.5, epsilon::EPSILON15);
    assert_approx!(oor.y, 0.25, epsilon::EPSILON15);
    assert_approx!(oor.z, 0.2, epsilon::EPSILON15);
}
