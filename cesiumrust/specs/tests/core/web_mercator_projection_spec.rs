//! Core/WebMercatorProjectionSpec.js → Rust integration tests (faithful port).
//!
//! Faithfully ports the original CesiumJS
//! `packages/engine/Specs/Core/WebMercatorProjectionSpec.js` (12 `it()` cases).
//! Reference values are used verbatim so the Rust implementation is verified
//! against the exact same ground truth as CesiumJS.
//!
//! Platform adaptations (documented, per the verification plan):
//! - CesiumJS "project3" / "unproject1" are "works with a result parameter"
//!   variants testing the JS memory-reuse API contract (`result === returnValue`).
//!   Rust returns owned values and has no result-parameter API, so those variants
//!   are subsumed by the owned-return tests below (identical computed values,
//!   single code path).
//! - CesiumJS "project throws without cartesian" actually invokes
//!   `projection.unproject()` with no argument, testing a runtime null-check.
//!   Rust's type system makes a missing argument unrepresentable (compile-time
//!   safety), so that error path has no Rust counterpart and is omitted.
//! - The "unproject is correct at corners" case passes `Cartesian2` inputs in
//!   CesiumJS; the Rust `unproject` takes a `DVec3`, so `z` is supplied as 0.0
//!   (only longitude/latitude are asserted, matching the original).
//! - `construct0` uses `new WebMercatorProjection()` which defaults to
//!   `Ellipsoid.default` (WGS84 in the test environment); the Rust equivalent of
//!   default construction is `WebMercatorProjection::wgs84()`.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::projection::{MapProjection, WebMercatorProjection};
use cesium_geospatial::math_utils::to_radians;
use cesium_specs::math_consts::{PI_OVER_FOUR, PI_OVER_TWO};
use cesium_specs::{assert_approx, epsilon};
use glam::DVec3;
use std::f64::consts::PI;

/// Web Mercator projected extent of the (square) world map, in meters.
const MAX_MERCATOR_EXTENT: f64 = 20037508.342787;

// "construct0"
#[test]
fn test_construct0() {
    let projection = WebMercatorProjection::wgs84();
    assert_eq!(projection.ellipsoid(), &Ellipsoid::WGS84);
}

// "construct1"
#[test]
fn test_construct1() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let projection = WebMercatorProjection::new(ellipsoid);
    assert_eq!(projection.ellipsoid(), &ellipsoid);
}

// "project0"
#[test]
fn test_project0() {
    let height = 10.0;
    let cartographic = Cartographic::from_radians(0.0, 0.0, height);
    let projection = WebMercatorProjection::wgs84();
    assert_eq!(
        projection.project(&cartographic),
        DVec3::new(0.0, 0.0, height)
    );
}

// "project1"
// expected equations from Wolfram MathWorld:
// http://mathworld.wolfram.com/MercatorProjection.html
#[test]
fn test_project1() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_radians(PI, PI_OVER_FOUR, 0.0);
    let expected = DVec3::new(
        ellipsoid.maximum_radius() * cartographic.longitude,
        ellipsoid.maximum_radius()
            * (PI / 4.0 + cartographic.latitude / 2.0).tan().ln(),
        0.0,
    );
    let projection = WebMercatorProjection::new(ellipsoid);
    assert_vec3_epsilon_proj(&projection.project(&cartographic), &expected, epsilon::EPSILON8);
}

// "project2"
#[test]
fn test_project2() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let cartographic = Cartographic::from_radians(-PI, PI_OVER_FOUR, 0.0);
    let expected = DVec3::new(
        ellipsoid.maximum_radius() * cartographic.longitude,
        ellipsoid.maximum_radius()
            * (PI / 4.0 + cartographic.latitude / 2.0).tan().ln(),
        0.0,
    );
    let projection = WebMercatorProjection::new(ellipsoid);
    assert_vec3_epsilon_proj(&projection.project(&cartographic), &expected, epsilon::EPSILON15);
}

// "unproject0"
#[test]
fn test_unproject0() {
    let cartographic = Cartographic::from_radians(PI_OVER_TWO, PI_OVER_FOUR, 12.0);
    let projection = WebMercatorProjection::wgs84();
    let projected = projection.project(&cartographic);
    let result = projection.unproject(projected);
    assert_approx!(result.longitude, cartographic.longitude, epsilon::EPSILON14);
    assert_approx!(result.latitude, cartographic.latitude, epsilon::EPSILON14);
    assert_approx!(result.height, cartographic.height, epsilon::EPSILON14);
}

// "unproject is correct at corners"
#[test]
fn test_unproject_is_correct_at_corners() {
    let projection = WebMercatorProjection::wgs84();

    let southwest = projection.unproject(DVec3::new(
        -MAX_MERCATOR_EXTENT,
        -MAX_MERCATOR_EXTENT,
        0.0,
    ));
    assert_approx!(southwest.longitude, -PI, epsilon::EPSILON12);
    assert_approx!(
        southwest.latitude,
        to_radians(-85.05112878),
        epsilon::EPSILON11
    );

    let southeast = projection.unproject(DVec3::new(
        MAX_MERCATOR_EXTENT,
        -MAX_MERCATOR_EXTENT,
        0.0,
    ));
    assert_approx!(southeast.longitude, PI, epsilon::EPSILON12);
    assert_approx!(
        southeast.latitude,
        to_radians(-85.05112878),
        epsilon::EPSILON11
    );

    let northeast = projection.unproject(DVec3::new(
        MAX_MERCATOR_EXTENT,
        MAX_MERCATOR_EXTENT,
        0.0,
    ));
    assert_approx!(northeast.longitude, PI, epsilon::EPSILON12);
    assert_approx!(
        northeast.latitude,
        to_radians(85.05112878),
        epsilon::EPSILON11
    );

    let northwest = projection.unproject(DVec3::new(
        -MAX_MERCATOR_EXTENT,
        MAX_MERCATOR_EXTENT,
        0.0,
    ));
    assert_approx!(northwest.longitude, -PI, epsilon::EPSILON12);
    assert_approx!(
        northwest.latitude,
        to_radians(85.05112878),
        epsilon::EPSILON11
    );
}

// "project is correct at corners."
#[test]
fn test_project_is_correct_at_corners() {
    let max_latitude = WebMercatorProjection::MAXIMUM_LATITUDE;
    let projection = WebMercatorProjection::wgs84();

    let southwest = projection.project(&Cartographic::from_radians(-PI, -max_latitude, 0.0));
    assert_approx!(southwest.x, -MAX_MERCATOR_EXTENT, epsilon::EPSILON3);
    assert_approx!(southwest.y, -MAX_MERCATOR_EXTENT, epsilon::EPSILON3);

    let southeast = projection.project(&Cartographic::from_radians(PI, -max_latitude, 0.0));
    assert_approx!(southeast.x, MAX_MERCATOR_EXTENT, epsilon::EPSILON3);
    assert_approx!(southeast.y, -MAX_MERCATOR_EXTENT, epsilon::EPSILON3);

    let northeast = projection.project(&Cartographic::from_radians(PI, max_latitude, 0.0));
    assert_approx!(northeast.x, MAX_MERCATOR_EXTENT, epsilon::EPSILON3);
    assert_approx!(northeast.y, MAX_MERCATOR_EXTENT, epsilon::EPSILON3);

    let northwest = projection.project(&Cartographic::from_radians(-PI, max_latitude, 0.0));
    assert_approx!(northwest.x, -MAX_MERCATOR_EXTENT, epsilon::EPSILON3);
    assert_approx!(northwest.y, MAX_MERCATOR_EXTENT, epsilon::EPSILON3);
}

// "projected y is clamped to valid latitude range."
#[test]
fn test_projected_y_is_clamped_to_valid_latitude_range() {
    let projection = WebMercatorProjection::wgs84();

    let south_pole = projection.project(&Cartographic::from_radians(0.0, -PI_OVER_TWO, 0.0));
    let south_limit = projection.project(&Cartographic::from_radians(
        0.0,
        -WebMercatorProjection::MAXIMUM_LATITUDE,
        0.0,
    ));
    assert_eq!(south_pole.y, south_limit.y);

    let north_pole = projection.project(&Cartographic::from_radians(0.0, PI_OVER_TWO, 0.0));
    let north_limit = projection.project(&Cartographic::from_radians(
        0.0,
        WebMercatorProjection::MAXIMUM_LATITUDE,
        0.0,
    ));
    assert_eq!(north_pole.y, north_limit.y);
}

/// Component-wise epsilon comparison for projected DVec3 results.
fn assert_vec3_epsilon_proj(actual: &DVec3, expected: &DVec3, eps: f64) {
    assert_approx!(actual.x, expected.x, eps);
    assert_approx!(actual.y, expected.y, eps);
    assert_approx!(actual.z, expected.z, eps);
}
