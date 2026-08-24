//! Tests for `cesium_core::EllipsoidRhumbLine`.

use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoid_rhumb_line::EllipsoidRhumbLine;

const EPSILON8: f64 = 1e-8;
const EPSILON12: f64 = 1e-12;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

#[test]
fn constructor_sets_defaults() {
    let line = EllipsoidRhumbLine::new(None, None, None, None);
    assert!(approx_eq(line.start().longitude, 0.0, EPSILON8));
    assert!(approx_eq(line.start().latitude, 0.0, EPSILON8));
    assert!(approx_eq(line.end().longitude, 0.0, EPSILON8));
    assert!(approx_eq(line.end().latitude, 0.0, EPSILON8));
    assert!(approx_eq(line.rhumb_distance(), 0.0, EPSILON8));
}

#[test]
fn constructor_with_start_end() {
    let start = Cartographic::new(0.0, 0.0, 0.0);
    let end = Cartographic::new(1.0, 1.0, 0.0);
    let line = EllipsoidRhumbLine::new(Some(start), Some(end), None, None);
    assert!(approx_eq(line.start().longitude, 0.0, EPSILON8));
    assert!(approx_eq(line.end().longitude, 1.0, EPSILON8));
    assert!(line.rhumb_distance() > 0.0);
}

#[test]
fn constructor_with_ellipsoid() {
    let start = Cartographic::new(0.0, 0.0, 0.0);
    let end = Cartographic::new(1.0, 1.0, 0.0);
    let line = EllipsoidRhumbLine::new(Some(start), Some(end), None, Some(Ellipsoid::WGS84));
    assert!(line.ellipsoid() == &Ellipsoid::WGS84);
}

#[test]
fn interpolate_using_fraction_at_zero_returns_start() {
    let start = Cartographic::new(0.1, 0.2, 100.0);
    let end = Cartographic::new(1.0, 1.0, 500.0);
    let line = EllipsoidRhumbLine::new(Some(start), Some(end), None, None);

    let result = line.interpolate_using_fraction(0.0);
    assert!(approx_eq(result.longitude, start.longitude, EPSILON8));
    assert!(approx_eq(result.latitude, start.latitude, EPSILON8));
    // Mirrors JS: `computeProperties` zeroes start/end heights (the rhumb
    // line is a surface curve), so the distance===0 clone carries height 0.
    assert!(approx_eq(result.height, 0.0, EPSILON8));
}

#[test]
fn interpolate_using_fraction_at_one_returns_end() {
    let start = Cartographic::new(0.1, 0.2, 100.0);
    let end = Cartographic::new(1.0, 1.0, 500.0);
    let line = EllipsoidRhumbLine::new(Some(start), Some(end), None, None);

    let result = line.interpolate_using_fraction(1.0);
    assert!(approx_eq(result.longitude, end.longitude, EPSILON8));
    assert!(approx_eq(result.latitude, end.latitude, EPSILON8));
    // Mirrors JS: interpolation never carries height (always 0 for
    // non-zero distance); the original spec never asserts height here.
    assert!(approx_eq(result.height, 0.0, EPSILON8));
}

#[test]
fn interpolate_using_fraction_at_half_returns_midpoint() {
    // Mirrors EllipsoidRhumbLineSpec.js "interpolates midpoint using
    // fraction": fifteenDegrees -> fortyfiveDegrees longitude at latitude 0
    // (constant-latitude line). The previous (0,0)->(2,2) radians data used
    // an invalid latitude (2.0 > PI/2), which yields NaN even in the JS
    // original.
    let fifteen_degrees = std::f64::consts::PI / 12.0;
    let thirty_degrees = std::f64::consts::PI / 6.0;
    let fortyfive_degrees = std::f64::consts::PI / 4.0;
    let start = Cartographic::new(fifteen_degrees, 0.0, 0.0);
    let end = Cartographic::new(fortyfive_degrees, 0.0, 0.0);
    let line = EllipsoidRhumbLine::new(Some(start), Some(end), None, None);

    let result = line.interpolate_using_fraction(0.5);
    assert!(approx_eq(result.longitude, thirty_degrees, EPSILON12));
    assert!(approx_eq(result.latitude, 0.0, EPSILON12));
}

#[test]
fn interpolate_using_surface_distance_zero_returns_start() {
    let start = Cartographic::new(0.1, 0.2, 0.0);
    let end = Cartographic::new(1.0, 1.0, 0.0);
    let line = EllipsoidRhumbLine::new(Some(start), Some(end), None, None);

    let result = line.interpolate_using_surface_distance(0.0);
    assert!(approx_eq(result.longitude, start.longitude, EPSILON8));
    assert!(approx_eq(result.latitude, start.latitude, EPSILON8));
}

#[test]
fn start_heading_is_computed() {
    let start = Cartographic::new(0.0, 0.0, 0.0);
    let end = Cartographic::new(1.0, 1.0, 0.0);
    let line = EllipsoidRhumbLine::new(Some(start), Some(end), None, None);
    // heading should be non-zero for a non-trivial rhumb line
    assert!(line.start_heading().abs() > 0.0);
}

#[test]
fn same_start_and_end_gives_zero_distance() {
    let pt = Cartographic::new(0.5, 0.5, 0.0);
    let line = EllipsoidRhumbLine::new(Some(pt), Some(pt), None, None);
    assert!(approx_eq(line.rhumb_distance(), 0.0, EPSILON8));
}
