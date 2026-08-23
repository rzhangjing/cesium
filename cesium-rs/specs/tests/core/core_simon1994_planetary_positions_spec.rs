//! Tests for `cesium_core::Simon1994PlanetaryPositions`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::julian_date::JulianDate;
use cesium_core::simon1994_planetary_positions::*;

fn magnitude(v: &Cartesian3) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

#[test]
fn compute_sun_position_returns_nonzero_vector() {
    let date = JulianDate::from_iso8601("2000-01-01T00:00:00Z").unwrap();
    let mut result = Cartesian3::default();
    compute_sun_position_in_earth_inertial_frame(&date, &mut result);
    assert!(magnitude(&result) > 0.0);
}

#[test]
fn compute_moon_position_returns_nonzero_vector() {
    let date = JulianDate::from_iso8601("2000-01-01T00:00:00Z").unwrap();
    let mut result = Cartesian3::default();
    compute_moon_position_in_earth_inertial_frame(&date, &mut result);
    assert!(magnitude(&result) > 0.0);
}

#[test]
fn sun_position_magnitude_is_approximately_1au() {
    let date = JulianDate::from_iso8601("2000-06-15T12:00:00Z").unwrap();
    let mut result = Cartesian3::default();
    compute_sun_position_in_earth_inertial_frame(&date, &mut result);
    let dist = magnitude(&result);
    // ~1 AU = 1.496e11 meters, allow 2% tolerance
    assert!(dist > 1.4e11 && dist < 1.6e11);
}

#[test]
fn moon_position_magnitude_is_approximately_correct() {
    let date = JulianDate::from_iso8601("2000-01-01T00:00:00Z").unwrap();
    let mut result = Cartesian3::default();
    compute_moon_position_in_earth_inertial_frame(&date, &mut result);
    let dist = magnitude(&result);
    // ~384400 km = 3.844e8 meters, allow 10% tolerance
    assert!(dist > 3.0e8 && dist < 5.0e8);
}
