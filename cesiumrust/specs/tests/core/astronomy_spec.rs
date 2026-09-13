//! Tests ported from CesiumJS:
//! - Simon1994PlanetaryPositionsSpec.js (3 A-class: sun position, moon position, sun rising east)
//! - Iau2000OrientationSpec.js (1 A-class: compute moon)
//! - IauOrientationAxesSpec.js (1 A-class: compute ICRF to Moon Fixed)

use cesium_geospatial::simon1994_planetary_positions::{
    compute_moon_position_in_earth_inertial_frame, compute_sun_position_in_earth_inertial_frame,
};
use cesium_geospatial::iau_orientation::{compute_moon, evaluate_icrf_to_fixed};
use cesium_time::JulianDate;

const EPSILON2: f64 = 1.0e-2;
const EPSILON3: f64 = 1.0e-3;
const EPSILON4: f64 = 1.0e-4;
const EPSILON13: f64 = 1.0e-13;

/// Helper: create JulianDate from day_number + seconds_of_day (TAI)
fn jd(day: f64, seconds: f64) -> JulianDate {
    JulianDate::with_time_standard(day, seconds, cesium_time::TimeStandard::TAI)
}

// ===== Simon1994PlanetaryPositions: computeSunPositionInEarthInertialFrame =====

#[test]
fn test_computes_correct_sun_position() {
    // J2000 epoch
    let date = jd(2451545.0, 0.0);
    let sun = compute_sun_position_in_earth_inertial_frame(&date);
    assert!((sun.x - 26500268539.790234).abs() < EPSILON2 * 26500268539.790234_f64.abs().max(1.0));
    assert!((sun.y - (-132756447253.27325)).abs() < EPSILON2 * 132756447253.27325);
    assert!((sun.z - (-57556483362.533806)).abs() < EPSILON2 * 57556483362.533806);

    // 2013-04-05
    let date = jd(2456401.5, 0.0);
    let sun = compute_sun_position_in_earth_inertial_frame(&date);
    assert!((sun.x - 131512388940.33589).abs() < EPSILON3 * 131512388940.33589);
    assert!((sun.y - 66661342667.949928).abs() < EPSILON3 * 66661342667.949928);
    assert!((sun.z - 28897975607.905258).abs() < EPSILON3 * 28897975607.905258);

    // 2012-03-01
    let date = jd(2455998.591667, 0.0);
    let sun = compute_sun_position_in_earth_inertial_frame(&date);
    assert!((sun.x - 147109989956.19534).abs() < EPSILON3 * 147109989956.19534);
    assert!((sun.y - (-19599996881.217579)).abs() < EPSILON3 * 19599996881.217579);
    assert!((sun.z - (-8497578102.7696457)).abs() < EPSILON3 * 8497578102.7696457);
}

// ===== Simon1994PlanetaryPositions: computeMoonPositionInEarthInertialFrame =====

#[test]
fn test_computes_correct_moon_position() {
    // J2000 epoch
    let date = jd(2451545.0, 0.0);
    let moon = compute_moon_position_in_earth_inertial_frame(&date);
    assert!((moon.x - (-291632410.61232185)).abs() < EPSILON4 * 291632410.61232185);
    assert!((moon.y - (-266522146.36821631)).abs() < EPSILON4 * 266522146.36821631);
    assert!((moon.z - (-75994518.081043154)).abs() < EPSILON4 * 75994518.081043154);

    // 2013-04-05
    let date = jd(2456401.5, 0.0);
    let moon = compute_moon_position_in_earth_inertial_frame(&date);
    assert!((moon.x - (-223792974.4736526)).abs() < EPSILON4 * 223792974.4736526);
    assert!((moon.y - 315772435.34490639).abs() < EPSILON4 * 315772435.34490639);
    assert!((moon.z - 97913011.236112773).abs() < EPSILON4 * 97913011.236112773);

    // 2012-03-01
    let date = jd(2455998.591667, 0.0);
    let moon = compute_moon_position_in_earth_inertial_frame(&date);
    assert!((moon.x - (-268426117.00202647)).abs() < EPSILON4 * 268426117.00202647);
    assert!((moon.y - (-220468861.73998192)).abs() < EPSILON4 * 220468861.73998192);
    assert!((moon.z - (-110670164.58446842)).abs() < EPSILON4 * 110670164.58446842);
}

// ===== Simon1994PlanetaryPositions: sun rising in east, setting in west =====

/// Simplified ICRF-to-Earth-Fixed rotation (Earth Rotation Angle only).
/// Sufficient to demonstrate diurnal motion.
fn icrf_to_fixed_rotation(jd_ut1: f64) -> glam::DMat3 {
    let du = jd_ut1 - 2451545.0;
    // IERS 2003 Earth Rotation Angle
    let theta = 2.0 * std::f64::consts::PI * (0.7790572732640 + 1.00273781191135448 * du);
    let (s, c) = theta.sin_cos();
    // Rotation about Z by -theta (ICRF → Fixed)
    glam::DMat3::from_cols_array(&[
        c, -s, 0.0,
        s,  c, 0.0,
        0.0, 0.0, 1.0,
    ])
}

#[test]
fn test_sun_rising_east_setting_west() {
    // Julian dates for 24 hours, starting from July 6th 2011 @ 01:00 UTC
    // July 6, 2011 00:00 UTC = JD 2455748.5
    let base_day = 2455748.5;
    let mut angles: Vec<f64> = Vec::new();

    for i in 1..25 {
        let date = jd(base_day, i as f64 * 3600.0);
        let position = compute_sun_position_in_earth_inertial_frame(&date);
        // Transform from inertial to Earth-fixed frame
        let rot = icrf_to_fixed_rotation(date.total_days());
        let fixed_pos = rot * position;
        let angle = fixed_pos.y.atan2(fixed_pos.x);
        // convertLongitudeRange: map to [-PI, PI]
        let mut lon = angle;
        while lon > std::f64::consts::PI {
            lon -= 2.0 * std::f64::consts::PI;
        }
        while lon < -std::f64::consts::PI {
            lon += 2.0 * std::f64::consts::PI;
        }
        angles.push(lon);
    }

    // Expect clockwise motion (angles decreasing) in Earth-fixed frame
    for i in 1..24 {
        assert!(
            angles[i] < angles[i - 1],
            "angles[{}] = {} should be < angles[{}] = {}",
            i, angles[i], i - 1, angles[i - 1]
        );
    }
}

// ===== Iau2000Orientation: ComputeMoon =====

#[test]
fn test_iau2000_compute_moon() {
    // date = new JulianDate(2451545.0, -32.184, TimeStandard.TAI)
    let date = jd(2451545.0, -32.184);
    let param = compute_moon(&date);

    // Expected results from STK Components
    let expected_right_ascension = 4.6575460830237914;
    let expected_declination = 1.1456533675897986;
    let expected_rotation = 0.71899299269222972;
    let expected_rotation_rate = 0.0000026518066425764541;

    assert_eq!(param.right_ascension, expected_right_ascension);
    assert_eq!(param.declination, expected_declination);
    assert_eq!(param.rotation, expected_rotation);
    assert_eq!(param.rotation_rate, expected_rotation_rate);
}

// ===== IauOrientationAxes: evaluate (ICRF to Moon Fixed) =====

#[test]
fn test_iau_orientation_axes_evaluate() {
    // date = new JulianDate(2451545.0, -32.184, TimeStandard.TAI)
    let date = jd(2451545.0, -32.184);
    let mtx = evaluate_icrf_to_fixed(&date);

    // Expected matrix from STK Components (column-major in CesiumJS)
    // Matrix3(col0row0, col0row1, col0row2, col1row0, col1row1, col1row2, col2row0, col2row1, col2row2)
    let expected = glam::DMat3::from_cols_array(&[
        0.784227052091917,    // col0, row0
        -0.62006191525085563, // col0, row1
        -0.022608671404182448, // col0, row2
        0.55784711246016394,  // col1, row0
        0.7205566654668133,   // col1, row1
        -0.41183090094261243, // col1, row2
        0.27165148607559436,  // col2, row0
        0.31035675134719942,  // col2, row1
        0.91097977859342938,  // col2, row2
    ]);

    let result_arr = mtx.to_cols_array();
    let expected_arr = expected.to_cols_array();
    for i in 0..9 {
        assert!(
            (result_arr[i] - expected_arr[i]).abs() < EPSILON13,
            "Matrix element [{}]: got {}, expected {}",
            i, result_arr[i], expected_arr[i]
        );
    }
}
