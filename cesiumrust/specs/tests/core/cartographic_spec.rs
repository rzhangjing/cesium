//! Core/CartographicSpec.js → Rust integration tests (faithful port).
//!
//! Faithfully ports the original CesiumJS `packages/engine/Specs/Core/CartographicSpec.js`
//! (24 `it()` cases). Reference values are used verbatim so the Rust implementation
//! is verified against the exact same ground truth as CesiumJS.
//!
//! Platform adaptations (documented, per the verification plan):
//! - CesiumJS "works with a result parameter" variants test the JS memory-reuse
//!   API contract (`returnedResult === result`). Rust returns owned values and has
//!   no result-parameter API, so those variants are subsumed by the owned-return
//!   tests below (identical computed values, single code path).
//! - CesiumJS "throws without longitude/latitude" and "throws when there is no
//!   cartesian" cases test runtime null-checks. Rust's type system makes null
//!   arguments unrepresentable (compile-time safety), so those error paths have
//!   no Rust counterpart. The "defaults altitude" half of those cases is ported
//!   by passing an explicit height of 0.0 (Rust has no optional parameters).
//! - `Ellipsoid.default` is a JS mutable-global pattern. The "uses default
//!   ellipsoid" tests set `Ellipsoid.default = Ellipsoid.MOON`; in Rust the
//!   ellipsoid is passed explicitly, so those tests pass `&Ellipsoid::MOON`
//!   directly (same conversion behavior, no global state).
//! - `clone` maps to Rust's derived `Clone`; `equals` maps to derived `PartialEq`.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils::to_radians;
use cesium_specs::{assert_vec3_epsilon, epsilon};
use glam::DVec3;

// --- Reference values from the original spec ---

#[allow(clippy::excessive_precision)]
fn surface_cartesian() -> DVec3 {
    DVec3::new(4094327.7921465295, 1909216.4044747739, 4487348.4088659193)
}
fn surface_cartographic() -> Cartographic {
    Cartographic::from_radians(to_radians(25.0), to_radians(45.0), 0.0)
}
#[allow(clippy::excessive_precision)]
fn moon_position() -> DVec3 {
    DVec3::new(1593514.338295244, 691991.9979835141, 20442.318221152018)
}
fn moon_cartographic() -> Cartographic {
    Cartographic::from_degrees(23.47315, 0.67416, 0.0)
}

// --- Constructor ---

// "default constructor sets expected properties"
#[test]
fn test_default_constructor_sets_expected_properties() {
    let c = Cartographic::default();
    assert_eq!(c.longitude, 0.0);
    assert_eq!(c.latitude, 0.0);
    assert_eq!(c.height, 0.0);
}

// "constructor sets expected properties from parameters"
#[test]
fn test_constructor_sets_expected_properties_from_parameters() {
    let c = Cartographic::from_radians(1.0, 2.0, 3.0);
    assert_eq!(c.longitude, 1.0);
    assert_eq!(c.latitude, 2.0);
    assert_eq!(c.height, 3.0);
}

// --- toCartesian ---

// "toCartesian conversion from Cartographic input to Cartesian3 output"
// Original asserts `Cartographic.toCartesian(c)` toEqual `ellipsoid.cartographicToCartesian(c)`.
#[test]
fn test_to_cartesian_conversion() {
    let lon = to_radians(150.0);
    let lat = to_radians(-40.0);
    let height = 100000.0;
    let ellipsoid = Ellipsoid::WGS84;
    let c = Cartographic::from_radians(lon, lat, height);
    let actual = Cartographic::to_cartesian(&c, &ellipsoid);
    let expected = ellipsoid.cartographic_to_cartesian(&c);
    assert_eq!(actual, expected);
}

// "toCartesian uses default ellipsoid"
// Original sets `Ellipsoid.default = Ellipsoid.MOON`; Rust passes MOON explicitly.
#[test]
fn test_to_cartesian_uses_moon_ellipsoid() {
    let cartographic = moon_cartographic();
    let position = Cartographic::to_cartesian(&cartographic, &Ellipsoid::MOON);
    assert_vec3_epsilon!(position, moon_position(), epsilon::EPSILON8);
}

// --- fromRadians ---

// "fromRadians works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_from_radians() {
    let c = Cartographic::from_radians(std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_4, 100.0);
    assert_eq!(c.longitude, std::f64::consts::FRAC_PI_2);
    assert_eq!(c.latitude, std::f64::consts::FRAC_PI_4);
    assert_eq!(c.height, 100.0);
}

// "fromRadians throws without longitude or latitude parameter but defaults altitude"
// The "throws" half has no Rust counterpart (type-safe, no null). The "defaults
// altitude" half is adapted: Rust has no optional parameters, so height 0.0 is
// passed explicitly (the JS default value).
#[test]
fn test_from_radians_defaults_altitude() {
    let c = Cartographic::from_radians(std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_4, 0.0);
    assert_eq!(c.longitude, std::f64::consts::FRAC_PI_2);
    assert_eq!(c.latitude, std::f64::consts::FRAC_PI_4);
    assert_eq!(c.height, 0.0);
}

// --- fromDegrees ---

// "fromDegrees works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_from_degrees() {
    let c = Cartographic::from_degrees(90.0, 45.0, 100.0);
    assert_eq!(c.longitude, std::f64::consts::FRAC_PI_2);
    assert_eq!(c.latitude, std::f64::consts::FRAC_PI_4);
    assert_eq!(c.height, 100.0);
}

// "fromDegrees throws without longitude or latitude parameter but defaults altitude"
// (adaptation as in test_from_radians_defaults_altitude)
#[test]
fn test_from_degrees_defaults_altitude() {
    let c = Cartographic::from_degrees(90.0, 45.0, 0.0);
    assert_eq!(c.longitude, std::f64::consts::FRAC_PI_2);
    assert_eq!(c.latitude, std::f64::consts::FRAC_PI_4);
    assert_eq!(c.height, 0.0);
}

// --- fromCartesian ---

// "fromCartesian works without a result parameter"
// (the "with a result parameter" variant is subsumed: Rust returns an owned value)
#[test]
fn test_from_cartesian() {
    let c = Cartographic::from_cartesian(surface_cartesian(), &Ellipsoid::WGS84).unwrap();
    assert!(c.equals_epsilon(&surface_cartographic(), epsilon::EPSILON8));
}

// "fromCartesian works without an ellipsoid"
// Original omits the ellipsoid (defaults to WGS84); Rust passes WGS84 explicitly.
#[test]
fn test_from_cartesian_default_ellipsoid_wgs84() {
    let c = Cartographic::from_cartesian(surface_cartesian(), &Ellipsoid::WGS84).unwrap();
    assert!(c.equals_epsilon(&surface_cartographic(), epsilon::EPSILON8));
}

// "fromCartesian uses default ellipsoid"
// Original sets `Ellipsoid.default = Ellipsoid.MOON`; Rust passes MOON explicitly.
#[test]
fn test_from_cartesian_uses_moon_ellipsoid() {
    let cartographic = Cartographic::from_cartesian(moon_position(), &Ellipsoid::MOON).unwrap();
    assert!(cartographic.equals_epsilon(&moon_cartographic(), epsilon::EPSILON8));
}

// "fromCartesian works with a value that is above the ellipsoid surface"
#[test]
fn test_from_cartesian_above_surface() {
    let cartographic1 = Cartographic::from_degrees(35.766989, 33.333602, 3000.0);
    // Cartesian3.fromRadians on the default (WGS84) ellipsoid.
    let cartesian1 = Ellipsoid::WGS84.cartographic_to_cartesian(&cartographic1);
    let cartographic2 = Cartographic::from_cartesian(cartesian1, &Ellipsoid::WGS84).unwrap();
    assert!(cartographic2.equals_epsilon(&cartographic1, epsilon::EPSILON8));
}

// "fromCartesian works with a value that is bellow the ellipsoid surface"
#[test]
fn test_from_cartesian_below_surface() {
    let cartographic1 = Cartographic::from_degrees(35.766989, 33.333602, -3000.0);
    let cartesian1 = Ellipsoid::WGS84.cartographic_to_cartesian(&cartographic1);
    let cartographic2 = Cartographic::from_cartesian(cartesian1, &Ellipsoid::WGS84).unwrap();
    assert!(cartographic2.equals_epsilon(&cartographic1, epsilon::EPSILON8));
}

// --- clone ---

// "clone without a result parameter"
// (the "with a result parameter" and "'this' result parameter" variants are
//  subsumed: Rust Clone always returns a fresh owned value)
#[test]
fn test_clone() {
    let cartographic = Cartographic::from_radians(1.0, 2.0, 3.0);
    let result = cartographic.clone();
    assert_ne!(
        &cartographic as *const Cartographic,
        &result as *const Cartographic
    );
    assert_eq!(cartographic, result);
}

// --- equals / equalsEpsilon ---

// "equals"
// (the `equals(undefined)` case is unrepresentable in Rust — type-safe equality)
#[test]
fn test_equals() {
    let cartographic = Cartographic::from_radians(1.0, 2.0, 3.0);
    assert!(cartographic == Cartographic::from_radians(1.0, 2.0, 3.0));
    assert!(cartographic != Cartographic::from_radians(2.0, 2.0, 3.0));
    assert!(cartographic != Cartographic::from_radians(2.0, 1.0, 3.0));
    assert!(cartographic != Cartographic::from_radians(1.0, 2.0, 4.0));
}

// "equalsEpsilon"
// (the `equalsEpsilon(undefined, 1)` case is unrepresentable in Rust)
#[test]
fn test_equals_epsilon() {
    let cartographic = Cartographic::from_radians(1.0, 2.0, 3.0);
    assert!(cartographic.equals_epsilon(&Cartographic::from_radians(1.0, 2.0, 3.0), 0.0));
    assert!(cartographic.equals_epsilon(&Cartographic::from_radians(1.0, 2.0, 3.0), 1.0));
    assert!(cartographic.equals_epsilon(&Cartographic::from_radians(2.0, 2.0, 3.0), 1.0));
    assert!(cartographic.equals_epsilon(&Cartographic::from_radians(1.0, 3.0, 3.0), 1.0));
    assert!(cartographic.equals_epsilon(&Cartographic::from_radians(1.0, 2.0, 4.0), 1.0));
    assert!(!cartographic.equals_epsilon(&Cartographic::from_radians(2.0, 2.0, 3.0), 0.99999));
    assert!(!cartographic.equals_epsilon(&Cartographic::from_radians(1.0, 3.0, 3.0), 0.99999));
    assert!(!cartographic.equals_epsilon(&Cartographic::from_radians(1.0, 2.0, 4.0), 0.99999));
}

// --- toString ---

// "toString"
#[test]
fn test_to_string() {
    let cartographic = Cartographic::from_radians(1.123, 2.345, 6.789);
    assert_eq!(format!("{}", cartographic), "(1.123, 2.345, 6.789)");
}
