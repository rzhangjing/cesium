//! Core/GeographicProjectionSpec.js → Rust integration tests (faithful port).
//!
//! Faithfully ports the original CesiumJS
//! `packages/engine/Specs/Core/GeographicProjectionSpec.js` (9 `it()` cases).
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
//! - `construct0` uses `new GeographicProjection()` which defaults to
//!   `Ellipsoid.default` (WGS84 in the test environment); the Rust equivalent of
//!   default construction is `GeographicProjection::wgs84()`.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::projection::{GeographicProjection, MapProjection};
use cesium_specs::math_consts::{PI_OVER_FOUR, PI_OVER_TWO};
use glam::DVec3;
use std::f64::consts::PI;

// "construct0"
#[test]
fn test_construct0() {
    let projection = GeographicProjection::wgs84();
    assert_eq!(projection.ellipsoid(), &Ellipsoid::WGS84);
}

// "construct1"
#[test]
fn test_construct1() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let projection = GeographicProjection::new(ellipsoid);
    assert_eq!(projection.ellipsoid(), &ellipsoid);
}

// "project0"
#[test]
fn test_project0() {
    let height = 10.0;
    let cartographic = Cartographic::from_radians(0.0, 0.0, height);
    let projection = GeographicProjection::wgs84();
    assert_eq!(
        projection.project(&cartographic),
        DVec3::new(0.0, 0.0, height)
    );
}

// "project1"
#[test]
fn test_project1() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_radians(PI, PI_OVER_TWO, 0.0);
    let expected = DVec3::new(
        PI * ellipsoid.radii().x,
        PI_OVER_TWO * ellipsoid.radii().x,
        0.0,
    );
    let projection = GeographicProjection::new(ellipsoid);
    assert_eq!(projection.project(&cartographic), expected);
}

// "project2"
#[test]
fn test_project2() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let cartographic = Cartographic::from_radians(-PI, PI_OVER_TWO, 0.0);
    let expected = DVec3::new(-PI, PI_OVER_TWO, 0.0);
    let projection = GeographicProjection::new(ellipsoid);
    assert_eq!(projection.project(&cartographic), expected);
}

// "unproject0"
#[test]
fn test_unproject0() {
    let cartographic = Cartographic::from_radians(PI_OVER_TWO, PI_OVER_FOUR, 12.0);
    let projection = GeographicProjection::wgs84();
    let projected = projection.project(&cartographic);
    assert_eq!(projection.unproject(projected), cartographic);
}
