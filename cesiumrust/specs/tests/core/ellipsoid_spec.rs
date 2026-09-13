//! Core/EllipsoidSpec.js → Rust integration tests (faithful port).
//!
//! Faithfully ports the original CesiumJS `packages/engine/Specs/Core/EllipsoidSpec.js`
//! (67 `it()` cases + createPackableSpecs). The original STK-Components reference
//! values are used verbatim so the Rust implementation is verified against the
//! exact same ground truth as CesiumJS.
//!
//! Platform adaptations (documented, per the verification plan):
//! - CesiumJS "works with a result parameter" variants test the JS memory-reuse
//!   API contract (`returnedResult === result`). Rust returns owned values and has
//!   no result-parameter API, so those variants are subsumed by the owned-return
//!   tests below (identical computed values, single code path).
//! - CesiumJS "throws with no <arg>" cases test runtime null-checks. Rust's type
//!   system makes null arguments unrepresentable (compile-time safety), so those
//!   error paths have no Rust counterpart.
//! - `Ellipsoid.default` static mutable setter is a JS global-state pattern with
//!   no Rust counterpart (ellipsoids are passed explicitly).
//! - `geocentricSurfaceNormal === Cartesian3.normalize` (function identity) is
//!   adapted to a behavioral test (returns the normalized vector).

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils::*;
use cesium_geospatial::rectangle::Rectangle;
use cesium_specs::{assert_approx, assert_vec2_epsilon, assert_vec3_epsilon, epsilon};
use glam::{DVec2, DVec3};

// --- Reference values from the original spec (computed using STK Components) ---

fn radii() -> DVec3 {
    DVec3::new(1.0, 2.0, 3.0)
}
fn radii_squared() -> DVec3 {
    DVec3::new(1.0, 4.0, 9.0)
}
fn radii_to_the_fourth() -> DVec3 {
    DVec3::new(1.0, 16.0, 81.0)
}
fn one_over_radii() -> DVec3 {
    DVec3::new(1.0, 0.5, 1.0 / 3.0)
}
fn one_over_radii_squared() -> DVec3 {
    DVec3::new(1.0, 0.25, 1.0 / 9.0)
}
const MINIMUM_RADIUS: f64 = 1.0;
const MAXIMUM_RADIUS: f64 = 3.0;

#[allow(clippy::excessive_precision)]
fn space_cartesian() -> DVec3 {
    DVec3::new(4582719.8827300891, -4582719.8827300882, 1725510.4250797231)
}
#[allow(clippy::excessive_precision)]
fn space_cartesian_geodetic_surface_normal() -> DVec3 {
    DVec3::new(
        0.6829975339864266,
        -0.68299753398642649,
        0.25889908678270795,
    )
}
fn space_cartographic() -> Cartographic {
    Cartographic::from_radians(to_radians(-45.0), to_radians(15.0), 330000.0)
}
#[allow(clippy::excessive_precision)]
fn space_cartographic_geodetic_surface_normal() -> DVec3 {
    DVec3::new(
        0.68301270189221941,
        -0.6830127018922193,
        0.25881904510252074,
    )
}
#[allow(clippy::excessive_precision)]
fn surface_cartesian() -> DVec3 {
    DVec3::new(4094327.7921465295, 1909216.4044747739, 4487348.4088659193)
}
fn surface_cartographic() -> Cartographic {
    Cartographic::from_radians(to_radians(25.0), to_radians(45.0), 0.0)
}

// --- Constructor / derived fields ---

// "default constructor creates zero Ellipsoid"
#[test]
fn test_default_constructor_creates_zero_ellipsoid() {
    let e = Ellipsoid::new(0.0, 0.0, 0.0);
    assert_eq!(e.radii(), DVec3::ZERO);
    assert_eq!(e.radii_squared(), DVec3::ZERO);
    assert_eq!(e.radii_to_the_fourth(), DVec3::ZERO);
    assert_eq!(e.one_over_radii(), DVec3::ZERO);
    assert_eq!(e.one_over_radii_squared(), DVec3::ZERO);
    assert_eq!(e.minimum_radius(), 0.0);
    assert_eq!(e.maximum_radius(), 0.0);
}

// "fromCartesian3 creates zero Ellipsoid with no parameters"
// (JS no-arg fromCartesian3 == zero radii; Rust equivalent is from_cartesian3(ZERO))
#[test]
fn test_from_cartesian3_creates_zero_ellipsoid() {
    let e = Ellipsoid::from_cartesian3(DVec3::ZERO);
    assert_eq!(e.radii(), DVec3::ZERO);
    assert_eq!(e.radii_squared(), DVec3::ZERO);
    assert_eq!(e.radii_to_the_fourth(), DVec3::ZERO);
    assert_eq!(e.one_over_radii(), DVec3::ZERO);
    assert_eq!(e.one_over_radii_squared(), DVec3::ZERO);
    assert_eq!(e.minimum_radius(), 0.0);
    assert_eq!(e.maximum_radius(), 0.0);
}

// "constructor computes correct values"
#[test]
fn test_constructor_computes_correct_values() {
    let e = Ellipsoid::new(radii().x, radii().y, radii().z);
    assert_eq!(e.radii(), radii());
    assert_eq!(e.radii_squared(), radii_squared());
    assert_eq!(e.radii_to_the_fourth(), radii_to_the_fourth());
    assert_eq!(e.one_over_radii(), one_over_radii());
    assert_eq!(e.one_over_radii_squared(), one_over_radii_squared());
    assert_eq!(e.minimum_radius(), MINIMUM_RADIUS);
    assert_eq!(e.maximum_radius(), MAXIMUM_RADIUS);
}

// "fromCartesian3 computes correct values"
#[test]
fn test_from_cartesian3_computes_correct_values() {
    let e = Ellipsoid::from_cartesian3(radii());
    assert_eq!(e.radii(), radii());
    assert_eq!(e.radii_squared(), radii_squared());
    assert_eq!(e.radii_to_the_fourth(), radii_to_the_fourth());
    assert_eq!(e.one_over_radii(), one_over_radii());
    assert_eq!(e.one_over_radii_squared(), one_over_radii_squared());
    assert_eq!(e.minimum_radius(), MINIMUM_RADIUS);
    assert_eq!(e.maximum_radius(), MAXIMUM_RADIUS);
}

// --- Geodetic surface normal ---

// "geodeticSurfaceNormalCartographic works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_geodetic_surface_normal_cartographic() {
    let e = Ellipsoid::WGS84;
    let result = e.geodetic_surface_normal_cartographic(&space_cartographic());
    assert_vec3_epsilon!(
        result,
        space_cartographic_geodetic_surface_normal(),
        epsilon::EPSILON15
    );
}

// "geodeticSurfaceNormal works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_geodetic_surface_normal() {
    let e = Ellipsoid::WGS84;
    let result = e.geodetic_surface_normal(space_cartesian()).unwrap();
    assert_vec3_epsilon!(
        result,
        space_cartesian_geodetic_surface_normal(),
        epsilon::EPSILON15
    );
}

// "geodeticSurfaceNormal returns undefined when given the origin"
#[test]
fn test_geodetic_surface_normal_returns_none_at_origin() {
    let e = Ellipsoid::WGS84;
    assert!(e.geodetic_surface_normal(DVec3::ZERO).is_none());
}

// --- cartographicToCartesian / cartesianToCartographic ---

// "cartographicToCartesian works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_cartographic_to_cartesian() {
    let e = Ellipsoid::WGS84;
    let result = e.cartographic_to_cartesian(&space_cartographic());
    assert_vec3_epsilon!(result, space_cartesian(), epsilon::EPSILON7);
}

// "cartographicArrayToCartesianArray works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned Vec)
#[test]
fn test_cartographic_array_to_cartesian_array() {
    let e = Ellipsoid::WGS84;
    let result = e.cartographic_array_to_cartesian_array(&[space_cartographic(), surface_cartographic()]);
    assert_eq!(result.len(), 2);
    assert_vec3_epsilon!(result[0], space_cartesian(), epsilon::EPSILON7);
    assert_vec3_epsilon!(result[1], surface_cartesian(), epsilon::EPSILON7);
}

// "cartesianToCartographic works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_cartesian_to_cartographic() {
    let e = Ellipsoid::WGS84;
    let result = e.cartesian_to_cartographic(surface_cartesian()).unwrap();
    assert!(result.equals_epsilon(&surface_cartographic(), epsilon::EPSILON8));
}

// "cartesianToCartographic works close to center"
// Original uses toEqual (exact equality) — verifies the bit-exact FP path.
#[test]
#[allow(clippy::excessive_precision)]
fn test_cartesian_to_cartographic_close_to_center() {
    let result = Ellipsoid::WGS84
        .cartesian_to_cartographic(DVec3::new(1e-50, 1e-60, 1e-70))
        .unwrap();
    assert_eq!(result.longitude, 9.999999999999999e-11);
    assert_eq!(result.latitude, 1.0067394967422763e-20);
    assert_eq!(result.height, -6378137.0);
}

// "cartesianToCartographic return undefined very close to center"
#[test]
fn test_cartesian_to_cartographic_none_very_close_to_center() {
    let e = Ellipsoid::WGS84;
    assert!(e
        .cartesian_to_cartographic(DVec3::new(1e-150, 1e-150, 1e-150))
        .is_none());
}

// "cartesianToCartographic return undefined at center"
#[test]
fn test_cartesian_to_cartographic_none_at_center() {
    let e = Ellipsoid::WGS84;
    assert!(e.cartesian_to_cartographic(DVec3::ZERO).is_none());
}

// "cartesianArrayToCartographicArray works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned Vec)
#[test]
fn test_cartesian_array_to_cartographic_array() {
    let e = Ellipsoid::WGS84;
    let result = e.cartesian_array_to_cartographic_array(&[space_cartesian(), surface_cartesian()]);
    assert_eq!(result.len(), 2);
    assert!(result[0]
        .unwrap()
        .equals_epsilon(&space_cartographic(), epsilon::EPSILON7));
    assert!(result[1]
        .unwrap()
        .equals_epsilon(&surface_cartographic(), epsilon::EPSILON7));
}

// --- scaleToGeodeticSurface ---

// "scaleToGeodeticSurface scaled in the x direction"
#[test]
fn test_scale_to_geodetic_surface_x() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let result = e.scale_to_geodetic_surface(DVec3::new(9.0, 0.0, 0.0)).unwrap();
    assert_eq!(result, DVec3::new(1.0, 0.0, 0.0));
}

// "scaleToGeodeticSurface scaled in the y direction"
#[test]
fn test_scale_to_geodetic_surface_y() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let result = e.scale_to_geodetic_surface(DVec3::new(0.0, 8.0, 0.0)).unwrap();
    assert_eq!(result, DVec3::new(0.0, 2.0, 0.0));
}

// "scaleToGeodeticSurface scaled in the z direction"
#[test]
fn test_scale_to_geodetic_surface_z() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let result = e.scale_to_geodetic_surface(DVec3::new(0.0, 0.0, 8.0)).unwrap();
    assert_eq!(result, DVec3::new(0.0, 0.0, 3.0));
}

// "scaleToGeodeticSurface works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
#[allow(clippy::excessive_precision)]
fn test_scale_to_geodetic_surface_general() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = DVec3::new(0.2680893773941855, 1.1160466902266495, 2.3559801120411263);
    let result = e.scale_to_geodetic_surface(DVec3::new(4.0, 5.0, 6.0)).unwrap();
    assert_vec3_epsilon!(result, expected, epsilon::EPSILON16);
}

// "scaleToGeodeticSurface returns undefined at center"
#[test]
fn test_scale_to_geodetic_surface_none_at_center() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    assert!(e.scale_to_geodetic_surface(DVec3::ZERO).is_none());
}

// --- scaleToGeocentricSurface ---

// "scaleToGeocentricSurface scaled in the x direction"
#[test]
fn test_scale_to_geocentric_surface_x() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let result = e.scale_to_geocentric_surface(DVec3::new(9.0, 0.0, 0.0)).unwrap();
    assert_eq!(result, DVec3::new(1.0, 0.0, 0.0));
}

// "scaleToGeocentricSurface scaled in the y direction"
#[test]
fn test_scale_to_geocentric_surface_y() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let result = e.scale_to_geocentric_surface(DVec3::new(0.0, 8.0, 0.0)).unwrap();
    assert_eq!(result, DVec3::new(0.0, 2.0, 0.0));
}

// "scaleToGeocentricSurface scaled in the z direction"
#[test]
fn test_scale_to_geocentric_surface_z() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let result = e.scale_to_geocentric_surface(DVec3::new(0.0, 0.0, 8.0)).unwrap();
    assert_eq!(result, DVec3::new(0.0, 0.0, 3.0));
}

// "scaleToGeocentricSurface works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
#[allow(clippy::excessive_precision)]
fn test_scale_to_geocentric_surface_general() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = DVec3::new(0.7807200583588266, 0.9759000729485333, 1.1710800875382399);
    let result = e.scale_to_geocentric_surface(DVec3::new(4.0, 5.0, 6.0)).unwrap();
    assert_vec3_epsilon!(result, expected, epsilon::EPSILON16);
}

// --- transformPositionToScaledSpace / FromScaledSpace ---

// "transformPositionToScaledSpace works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_transform_position_to_scaled_space() {
    let e = Ellipsoid::new(2.0, 3.0, 4.0);
    let result = e.transform_position_to_scaled_space(DVec3::new(4.0, 6.0, 8.0));
    assert_vec3_epsilon!(result, DVec3::new(2.0, 2.0, 2.0), epsilon::EPSILON16);
}

// "transformPositionFromScaledSpace works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_transform_position_from_scaled_space() {
    let e = Ellipsoid::new(2.0, 3.0, 4.0);
    let result = e.transform_position_from_scaled_space(DVec3::new(2.0, 2.0, 2.0));
    assert_vec3_epsilon!(result, DVec3::new(4.0, 6.0, 8.0), epsilon::EPSILON16);
}

// --- equals / toString ---

// "equals works in all cases"
// (the `equals(undefined)` case is unrepresentable in Rust — type-safe equality)
#[test]
fn test_equals() {
    let e = Ellipsoid::new(1.0, 0.0, 0.0);
    assert!(e == Ellipsoid::new(1.0, 0.0, 0.0));
    assert!(e != Ellipsoid::new(1.0, 1.0, 0.0));
}

// "toString produces expected values"
#[test]
fn test_to_string() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    assert_eq!(format!("{}", e), "(1, 2, 3)");
}

// --- constructor validation ---

// "constructor throws if x less than 0"
#[test]
#[should_panic]
fn test_constructor_throws_x_negative() {
    let _ = Ellipsoid::new(-1.0, 0.0, 0.0);
}

// "constructor throws if y less than 0"
#[test]
#[should_panic]
fn test_constructor_throws_y_negative() {
    let _ = Ellipsoid::new(0.0, -1.0, 0.0);
}

// "constructor throws if z less than 0"
#[test]
#[should_panic]
fn test_constructor_throws_z_negative() {
    let _ = Ellipsoid::new(0.0, 0.0, -1.0);
}

// "expect Ellipsoid.geocentricSurfaceNormal is be Cartesian3.normalize"
// Adapted from a function-identity check to a behavioral check (Rust has no
// function-identity semantics): the geocentric surface normal is the normalized
// position vector.
#[test]
fn test_geocentric_surface_normal_is_normalize() {
    let e = Ellipsoid::WGS84;
    let p = space_cartesian();
    assert_vec3_epsilon!(e.geocentric_surface_normal(p), p.normalize(), epsilon::EPSILON15);
}

// --- clone ---

// "clone copies any object with the proper structure"
// "clone uses result parameter if provided" (subsumed: Rust Clone returns owned value)
#[test]
fn test_clone() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let cloned = e.clone();
    assert_eq!(cloned, e);
    assert_eq!(cloned.radii(), radii());
    assert_eq!(cloned.radii_squared(), radii_squared());
    assert_eq!(cloned.minimum_radius(), MINIMUM_RADIUS);
    assert_eq!(cloned.maximum_radius(), MAXIMUM_RADIUS);
}

// --- getSurfaceNormalIntersectionWithZAxis ---

// "getSurfaceNormalIntersectionWithZAxis throws if the ellipsoid is not an
//  ellipsoid of revolution"
#[test]
#[should_panic]
fn test_surface_normal_intersection_throws_not_revolution() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let _ = e.get_surface_normal_intersection_with_z_axis(DVec3::ZERO, None);
}

// "getSurfaceNormalIntersectionWithZAxis throws if the ellipsoid has radii.z === 0"
// (original uses Ellipsoid(1,2,0); the revolution check fires first — the point is
//  that a degenerate ellipsoid panics)
#[test]
#[should_panic]
fn test_surface_normal_intersection_throws_z_zero() {
    let e = Ellipsoid::new(1.0, 2.0, 0.0);
    let _ = e.get_surface_normal_intersection_with_z_axis(DVec3::ZERO, None);
}

// "getSurfaceNormalIntersectionWithZAxis works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_surface_normal_intersection_works() {
    let e = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees(35.23, 33.23, 0.0);
    let cartesian_on_the_surface = e.cartographic_to_cartesian(&cartographic);
    let result = e.get_surface_normal_intersection_with_z_axis(cartesian_on_the_surface, None);
    assert!(result.is_some());
}

// "getSurfaceNormalIntersectionWithZAxis returns undefined if the result is outside
//  the ellipsoid with buffer parameter"
#[test]
fn test_surface_normal_intersection_none_with_buffer() {
    let e = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees(35.23, 33.23, 0.0);
    let cartesian_on_the_surface = e.cartographic_to_cartesian(&cartographic);
    let result = e.get_surface_normal_intersection_with_z_axis(
        cartesian_on_the_surface,
        Some(e.radii().z),
    );
    assert!(result.is_none());
}

// "getSurfaceNormalIntersectionWithZAxis returns undefined if the result is outside
//  the ellipsoid without buffer parameter"
#[test]
fn test_surface_normal_intersection_none_without_buffer() {
    let major_axis = 10.0;
    let minor_axis = 1.0;
    let e = Ellipsoid::new(major_axis, major_axis, minor_axis);
    let cartographic = Cartographic::from_degrees(45.0, 90.0, 0.0);
    let cartesian_on_the_surface = e.cartographic_to_cartesian(&cartographic);
    let result = e.get_surface_normal_intersection_with_z_axis(cartesian_on_the_surface, None);
    assert!(result.is_none());
}

// "getSurfaceNormalIntersectionWithZAxis returns a result that is equal to a value
//  that computed in a different way"
#[test]
fn test_surface_normal_intersection_matches_alternate_computation() {
    let e = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees(35.23, 33.23, 0.0);
    let mut cartesian_on_the_surface = e.cartographic_to_cartesian(&cartographic);
    let surface_normal = e.geodetic_surface_normal(cartesian_on_the_surface).unwrap();
    let magnitude = cartesian_on_the_surface.x / surface_normal.x;

    let expected = DVec3::new(
        0.0,
        0.0,
        cartesian_on_the_surface.z - surface_normal.z * magnitude,
    );
    let result = e
        .get_surface_normal_intersection_with_z_axis(cartesian_on_the_surface, None)
        .unwrap();
    assert_vec3_epsilon!(result, expected, epsilon::EPSILON8);

    // at the equator
    cartesian_on_the_surface = DVec3::new(e.radii().x, 0.0, 0.0);
    let result = e
        .get_surface_normal_intersection_with_z_axis(cartesian_on_the_surface, None)
        .unwrap();
    assert_vec3_epsilon!(result, DVec3::ZERO, epsilon::EPSILON8);
}

// "getSurfaceNormalIntersectionWithZAxis returns a result that when it's used as an
//  origin for a vector with the surface normal direction it produces an accurate
//  cartographic"
#[test]
fn test_surface_normal_intersection_produces_accurate_cartographic() {
    let e = Ellipsoid::WGS84;

    // general position
    let mut cartographic = Cartographic::from_degrees(35.23, 33.23, 0.0);
    let mut cartesian_on_the_surface = e.cartographic_to_cartesian(&cartographic);
    let mut surface_normal = e.geodetic_surface_normal(cartesian_on_the_surface).unwrap();
    let mut result = e
        .get_surface_normal_intersection_with_z_axis(cartesian_on_the_surface, None)
        .unwrap();
    let surface_normal_with_length = surface_normal * e.maximum_radius();
    let position = result + surface_normal_with_length;
    let mut result_cartographic = e.cartesian_to_cartographic(position).unwrap();
    result_cartographic.height = 0.0;
    assert!(result_cartographic.equals_epsilon(&cartographic, epsilon::EPSILON8));

    // at the north pole
    cartographic = Cartographic::from_degrees(0.0, 90.0, 0.0);
    cartesian_on_the_surface = DVec3::new(0.0, 0.0, e.radii().z);
    surface_normal = e.geodetic_surface_normal(cartesian_on_the_surface).unwrap();
    result = e
        .get_surface_normal_intersection_with_z_axis(cartesian_on_the_surface, None)
        .unwrap();
    let surface_normal_with_length = surface_normal * e.maximum_radius();
    let position = result + surface_normal_with_length;
    let mut result_cartographic = e.cartesian_to_cartographic(position).unwrap();
    result_cartographic.height = 0.0;
    assert!(result_cartographic.equals_epsilon(&cartographic, epsilon::EPSILON8));
}

// --- getLocalCurvature ---

// "getLocalCurvature returns expected values at the equator"
#[test]
fn test_local_curvature_at_equator() {
    let e = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees(0.0, 0.0, 0.0);
    let cartesian_on_the_surface = e.cartographic_to_cartesian(&cartographic);
    let result = e.get_local_curvature(cartesian_on_the_surface).unwrap();
    let expected = DVec2::new(
        1.0 / e.maximum_radius(),
        e.maximum_radius() / (e.minimum_radius() * e.minimum_radius()),
    );
    assert_vec2_epsilon!(result, expected, epsilon::EPSILON8);
}

// "getLocalCurvature returns expected values at the north pole"
#[test]
fn test_local_curvature_at_north_pole() {
    let e = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees(0.0, 90.0, 0.0);
    let cartesian_on_the_surface = e.cartographic_to_cartesian(&cartographic);
    let result = e.get_local_curvature(cartesian_on_the_surface).unwrap();
    let semi_latus_rectum = (e.maximum_radius() * e.maximum_radius()) / e.minimum_radius();
    let expected = DVec2::new(1.0 / semi_latus_rectum, 1.0 / semi_latus_rectum);
    assert_vec2_epsilon!(result, expected, epsilon::EPSILON8);
}

// --- squaredXOverSquaredZ ---

// "ellipsoid is initialized with _squaredXOverSquaredZ property"
#[test]
fn test_squared_x_over_squared_z() {
    let e = Ellipsoid::new(4.0, 4.0, 3.0);
    let expected = e.radii_squared().x / e.radii_squared().z;
    assert_eq!(e.squared_x_over_squared_z(), expected);
}

// --- surfaceArea ---

// "computes surfaceArea"
#[test]
fn test_surface_area() {
    let full = Rectangle::new(-PI_F64, -PI_OVER_TWO, PI_F64, PI_OVER_TWO);

    // area of an oblate spheroid
    let e = Ellipsoid::new(4.0, 4.0, 3.0);
    let a2 = e.radii_squared().x;
    let c2 = e.radii_squared().z;
    let ecc = (1.0 - c2 / a2).sqrt();
    let area = TWO_PI * a2 + PI_F64 * (c2 / ecc) * ((1.0 + ecc) / (1.0 - ecc)).ln();
    assert_approx!(e.surface_area(&full), area, epsilon::EPSILON3);

    // area of a prolate spheroid
    let e = Ellipsoid::new(3.0, 3.0, 4.0);
    let a2 = e.radii_squared().x;
    let c2 = e.radii_squared().z;
    let ecc = (1.0 - a2 / c2).sqrt();
    let a = e.radii().x;
    let c = e.radii().z;
    let area = TWO_PI * a2 + TWO_PI * ((a * c) / ecc) * ecc.asin();
    assert_approx!(e.surface_area(&full), area, epsilon::EPSILON3);
}

// --- Packable (createPackableSpecs) ---

// createPackableSpecs: packedLength / pack / unpack round-trip.
#[test]
fn test_packed_length() {
    assert_eq!(Ellipsoid::PACKED_LENGTH, 3);
}

#[test]
fn test_pack_unpack_roundtrip() {
    let e = Ellipsoid::WGS84;
    let mut array = [0.0f64; 3];
    e.pack(&mut array, 0);
    assert_eq!(array[0], Ellipsoid::WGS84.radii().x);
    assert_eq!(array[1], Ellipsoid::WGS84.radii().y);
    assert_eq!(array[2], Ellipsoid::WGS84.radii().z);

    let unpacked = Ellipsoid::unpack(&array, 0);
    assert_eq!(unpacked, e);
}
