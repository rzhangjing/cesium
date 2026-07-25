//! Core/TransformsSpec.js, HeadingPitchRollSpec.js, HeadingPitchRangeSpec.js,
//! TranslationRotationScaleSpec.js → Rust integration tests

use cesium_geospatial::transforms::{
    compute_fixed_to_icrf_matrix, compute_icrf_to_fixed_matrix, east_north_up_to_fixed_frame,
    heading_pitch_roll_quaternion, heading_pitch_roll_to_fixed_frame, look_at,
    HeadingPitchRange, HeadingPitchRoll, TranslationRotationScale,
};
use cesium_geospatial::Ellipsoid;
use cesium_specs::{assert_approx, assert_vec3_epsilon, epsilon};
use glam::{DMat3, DQuat, DVec3};
use std::f64::consts::PI;

// === HeadingPitchRoll ===

#[test]
fn test_hpr_new() {
    let hpr = HeadingPitchRoll::new(0.1, 0.2, 0.3);
    assert_approx!(hpr.heading, 0.1, epsilon::EPSILON15);
    assert_approx!(hpr.pitch, 0.2, epsilon::EPSILON15);
    assert_approx!(hpr.roll, 0.3, epsilon::EPSILON15);
}

#[test]
fn test_hpr_from_degrees() {
    let hpr = HeadingPitchRoll::from_degrees(90.0, 45.0, 0.0);
    assert_approx!(hpr.heading, PI / 2.0, epsilon::EPSILON10);
    assert_approx!(hpr.pitch, PI / 4.0, epsilon::EPSILON10);
    assert_approx!(hpr.roll, 0.0, epsilon::EPSILON15);
}

#[test]
fn test_hpr_to_quaternion_identity() {
    let hpr = HeadingPitchRoll::new(0.0, 0.0, 0.0);
    let q = hpr.to_quaternion();
    assert_approx!(q.w, 1.0, epsilon::EPSILON10);
    assert_approx!(q.x, 0.0, epsilon::EPSILON10);
    assert_approx!(q.y, 0.0, epsilon::EPSILON10);
    assert_approx!(q.z, 0.0, epsilon::EPSILON10);
}

#[test]
fn test_hpr_to_quaternion_heading_90() {
    let hpr = HeadingPitchRoll::new(PI / 2.0, 0.0, 0.0);
    let q = hpr.to_quaternion();
    // 90° about Z axis
    assert_approx!(q.z, (PI / 4.0).sin(), epsilon::EPSILON10);
    assert_approx!(q.w, (PI / 4.0).cos(), epsilon::EPSILON10);
}

#[test]
fn test_hpr_to_quaternion_pitch_90() {
    let hpr = HeadingPitchRoll::new(0.0, PI / 2.0, 0.0);
    let q = hpr.to_quaternion();
    // 90° about Y axis
    assert_approx!(q.y, (PI / 4.0).sin(), epsilon::EPSILON10);
    assert_approx!(q.w, (PI / 4.0).cos(), epsilon::EPSILON10);
}

// === HeadingPitchRange ===

#[test]
fn test_hpr_range_new() {
    let hpr_range = HeadingPitchRange::new(0.5, -0.3, 1000.0);
    assert_approx!(hpr_range.heading, 0.5, epsilon::EPSILON15);
    assert_approx!(hpr_range.pitch, -0.3, epsilon::EPSILON15);
    assert_approx!(hpr_range.range, 1000.0, epsilon::EPSILON15);
}

// === TranslationRotationScale ===

#[test]
fn test_trs_new() {
    let t = DVec3::new(1.0, 2.0, 3.0);
    let r = DQuat::IDENTITY;
    let s = DVec3::new(2.0, 2.0, 2.0);
    let trs = TranslationRotationScale::new(t, r, s);
    assert_vec3_epsilon!(trs.translation, t, epsilon::EPSILON15);
    assert_vec3_epsilon!(trs.scale, s, epsilon::EPSILON15);
}

#[test]
fn test_trs_to_matrix4_identity() {
    let trs = TranslationRotationScale::new(DVec3::ZERO, DQuat::IDENTITY, DVec3::ONE);
    let mat = trs.to_matrix4();
    assert!(mat.abs_diff_eq(glam::DMat4::IDENTITY, 1e-10));
}

#[test]
fn test_trs_to_matrix4_translation() {
    let trs = TranslationRotationScale::new(
        DVec3::new(5.0, 10.0, 15.0),
        DQuat::IDENTITY,
        DVec3::ONE,
    );
    let mat = trs.to_matrix4();
    assert_vec3_epsilon!(mat.w_axis.truncate(), DVec3::new(5.0, 10.0, 15.0), epsilon::EPSILON10);
}

#[test]
fn test_trs_to_matrix4_scale() {
    let trs = TranslationRotationScale::new(
        DVec3::ZERO,
        DQuat::IDENTITY,
        DVec3::new(2.0, 3.0, 4.0),
    );
    let mat = trs.to_matrix4();
    assert_approx!(mat.x_axis.x, 2.0, epsilon::EPSILON10);
    assert_approx!(mat.y_axis.y, 3.0, epsilon::EPSILON10);
    assert_approx!(mat.z_axis.z, 4.0, epsilon::EPSILON10);
}

// === ENU Frame ===

#[test]
fn test_enu_at_equator_prime_meridian() {
    let ellipsoid = Ellipsoid::WGS84;
    let origin = DVec3::new(ellipsoid.radii().x, 0.0, 0.0);
    let frame = east_north_up_to_fixed_frame(origin, &ellipsoid);

    let east = frame.x_axis.truncate();
    let north = frame.y_axis.truncate();
    let up = frame.z_axis.truncate();

    // At (lat=0, lon=0): East=(0,1,0), North=(0,0,1), Up=(1,0,0)
    assert_vec3_epsilon!(east, DVec3::Y, epsilon::EPSILON10);
    assert_vec3_epsilon!(north, DVec3::Z, epsilon::EPSILON10);
    assert_vec3_epsilon!(up, DVec3::X, epsilon::EPSILON10);
}

#[test]
fn test_enu_at_equator_90_degrees() {
    let ellipsoid = Ellipsoid::WGS84;
    let origin = DVec3::new(0.0, ellipsoid.radii().x, 0.0);
    let frame = east_north_up_to_fixed_frame(origin, &ellipsoid);

    let east = frame.x_axis.truncate();
    let up = frame.z_axis.truncate();

    // At (lat=0, lon=90): East=(-1,0,0), Up=(0,1,0)
    assert_vec3_epsilon!(east, DVec3::new(-1.0, 0.0, 0.0), epsilon::EPSILON10);
    assert_vec3_epsilon!(up, DVec3::Y, epsilon::EPSILON10);
}

#[test]
fn test_enu_orthogonality() {
    let ellipsoid = Ellipsoid::WGS84;
    let origin = DVec3::new(4000000.0, 3000000.0, 2000000.0);
    let frame = east_north_up_to_fixed_frame(origin, &ellipsoid);

    let east = frame.x_axis.truncate();
    let north = frame.y_axis.truncate();
    let up = frame.z_axis.truncate();

    // All axes should be orthogonal
    assert_approx!(east.dot(north), 0.0, epsilon::EPSILON10);
    assert_approx!(east.dot(up), 0.0, epsilon::EPSILON10);
    assert_approx!(north.dot(up), 0.0, epsilon::EPSILON10);

    // All axes should be unit length
    assert_approx!(east.length(), 1.0, epsilon::EPSILON10);
    assert_approx!(north.length(), 1.0, epsilon::EPSILON10);
    assert_approx!(up.length(), 1.0, epsilon::EPSILON10);
}

// === HeadingPitchRoll Quaternion at Origin ===

#[test]
fn test_hpr_quaternion_at_origin() {
    let ellipsoid = Ellipsoid::WGS84;
    let origin = DVec3::new(ellipsoid.radii().x, 0.0, 0.0);
    let hpr = HeadingPitchRoll::new(0.0, 0.0, 0.0);
    let q = heading_pitch_roll_quaternion(&hpr, origin, &ellipsoid);
    // Should be a valid unit quaternion
    assert_approx!(q.length(), 1.0, epsilon::EPSILON10);
}

#[test]
fn test_hpr_to_fixed_frame() {
    let ellipsoid = Ellipsoid::WGS84;
    let origin = DVec3::new(ellipsoid.radii().x, 0.0, 0.0);
    let hpr = HeadingPitchRoll::new(0.0, 0.0, 0.0);
    let frame = heading_pitch_roll_to_fixed_frame(&hpr, origin, &ellipsoid);
    // Translation should be the origin
    assert_vec3_epsilon!(frame.w_axis.truncate(), origin, epsilon::EPSILON6);
}

// === ICRF Transforms ===

#[test]
fn test_icrf_to_fixed_is_rotation() {
    let j2000_seconds = 2451545.0 * 86400.0;
    let mat = compute_icrf_to_fixed_matrix(j2000_seconds).unwrap();
    let det = mat.determinant();
    assert_approx!(det, 1.0, epsilon::EPSILON10);
}

#[test]
fn test_fixed_to_icrf_is_inverse() {
    let seconds = 2451545.0 * 86400.0 + 3600.0;
    let icrf_to_fixed = compute_icrf_to_fixed_matrix(seconds).unwrap();
    let fixed_to_icrf = compute_fixed_to_icrf_matrix(seconds).unwrap();
    let product = icrf_to_fixed * fixed_to_icrf;
    assert!(product.abs_diff_eq(DMat3::IDENTITY, 1e-10));
}

// === lookAt ===

#[test]
fn test_look_at_basic() {
    let eye = DVec3::new(0.0, 0.0, 10.0);
    let target = DVec3::ZERO;
    let up = DVec3::Y;
    let mat = look_at(eye, target, up);
    // Translation column should be eye
    assert_vec3_epsilon!(mat.w_axis.truncate(), eye, epsilon::EPSILON10);
}
