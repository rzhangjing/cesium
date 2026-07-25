//! Core/MathSpec.js → Rust integration tests
//! Tests for cesium_geospatial::math_utils

use cesium_geospatial::math_utils::*;
use cesium_specs::{assert_approx, epsilon};
use std::f64::consts::PI;

#[test]
fn test_math_constants() {
    assert_approx!(PI_F64, PI, epsilon::EPSILON15);
    assert_approx!(TWO_PI, 2.0 * PI, epsilon::EPSILON15);
    assert_approx!(PI_OVER_TWO, PI / 2.0, epsilon::EPSILON15);
    assert_approx!(PI_OVER_THREE, PI / 3.0, epsilon::EPSILON15);
    assert_approx!(PI_OVER_FOUR, PI / 4.0, epsilon::EPSILON15);
    assert_approx!(PI_OVER_SIX, PI / 6.0, epsilon::EPSILON15);
    assert_approx!(THREE_PI_OVER_TWO, 3.0 * PI / 2.0, epsilon::EPSILON15);
}

#[test]
fn test_epsilon_constants() {
    assert_eq!(EPSILON1, 1e-1);
    assert_eq!(EPSILON5, 1e-5);
    assert_eq!(EPSILON10, 1e-10);
    assert_eq!(EPSILON15, 1e-15);
    assert_eq!(EPSILON20, 1e-20);
}

#[test]
fn test_to_radians() {
    assert_approx!(to_radians(0.0), 0.0, EPSILON15);
    assert_approx!(to_radians(90.0), PI_OVER_TWO, EPSILON15);
    assert_approx!(to_radians(180.0), PI, EPSILON15);
    assert_approx!(to_radians(270.0), THREE_PI_OVER_TWO, EPSILON15);
    assert_approx!(to_radians(360.0), TWO_PI, EPSILON15);
    assert_approx!(to_radians(-90.0), -PI_OVER_TWO, EPSILON15);
}

#[test]
fn test_to_degrees() {
    assert_approx!(to_degrees(0.0), 0.0, EPSILON13);
    assert_approx!(to_degrees(PI_OVER_TWO), 90.0, EPSILON13);
    assert_approx!(to_degrees(PI), 180.0, EPSILON13);
    assert_approx!(to_degrees(TWO_PI), 360.0, EPSILON13);
    assert_approx!(to_degrees(-PI), -180.0, EPSILON13);
}

#[test]
fn test_to_radians_to_degrees_roundtrip() {
    let angle = 45.0;
    assert_approx!(to_degrees(to_radians(angle)), angle, EPSILON13);
}

#[test]
fn test_clamp() {
    assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
    assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
    assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    assert_eq!(clamp(0.0, 0.0, 10.0), 0.0);
    assert_eq!(clamp(10.0, 0.0, 10.0), 10.0);
}

#[test]
fn test_sign() {
    assert_eq!(sign(5.0), 1.0);
    assert_eq!(sign(-5.0), -1.0);
    assert_eq!(sign(0.0), 0.0);
}

#[test]
fn test_sign_not_zero() {
    assert_eq!(sign_not_zero(5.0), 1.0);
    assert_eq!(sign_not_zero(-5.0), -1.0);
    assert_eq!(sign_not_zero(0.0), 1.0);
}

#[test]
fn test_lerp() {
    assert_approx!(lerp(0.0, 10.0, 0.0), 0.0, EPSILON15);
    assert_approx!(lerp(0.0, 10.0, 0.5), 5.0, EPSILON15);
    assert_approx!(lerp(0.0, 10.0, 1.0), 10.0, EPSILON15);
    assert_approx!(lerp(2.0, 8.0, 0.25), 3.5, EPSILON15);
}

#[test]
fn test_negative_pi_to_pi() {
    assert_approx!(negative_pi_to_pi(0.0), 0.0, EPSILON14);
    assert_approx!(negative_pi_to_pi(PI), PI, EPSILON14);
    assert_approx!(negative_pi_to_pi(-PI), -PI, EPSILON14);
    assert_approx!(negative_pi_to_pi(THREE_PI_OVER_TWO), -PI_OVER_TWO, EPSILON14);
    assert_approx!(negative_pi_to_pi(TWO_PI), 0.0, EPSILON14);
    // 3*PI normalizes to either PI or -PI (both valid)
    let result = negative_pi_to_pi(3.0 * PI);
    assert!(result.abs() - PI < EPSILON14 || (result - PI).abs() < EPSILON14);
}

#[test]
fn test_zero_to_two_pi() {
    assert_approx!(zero_to_two_pi(0.0), 0.0, EPSILON14);
    assert_approx!(zero_to_two_pi(PI), PI, EPSILON14);
    assert_approx!(zero_to_two_pi(-PI_OVER_TWO), THREE_PI_OVER_TWO, EPSILON14);
    assert_approx!(zero_to_two_pi(TWO_PI + PI_OVER_TWO), PI_OVER_TWO, EPSILON14);
}

#[test]
fn test_equals_epsilon() {
    assert!(equals_epsilon(1.0, 1.0, 0.0, EPSILON15));
    assert!(equals_epsilon(1.0, 1.0 + 1e-16, 0.0, EPSILON15));
    assert!(!equals_epsilon(1.0, 2.0, 0.0, EPSILON15));
    // Relative epsilon
    assert!(equals_epsilon(1000.0, 1000.1, EPSILON3, 0.0));
}

#[test]
fn test_factorial() {
    assert_eq!(factorial(0), 1);
    assert_eq!(factorial(1), 1);
    assert_eq!(factorial(5), 120);
    assert_eq!(factorial(10), 3628800);
}

#[test]
fn test_chord_length() {
    // chord = 2 * r * sin(angle/2)
    assert_approx!(chord_length(PI, 1.0), 2.0, EPSILON14);
    assert_approx!(chord_length(PI_OVER_TWO, 1.0), std::f64::consts::SQRT_2, EPSILON14);
    assert_approx!(chord_length(0.0, 1.0), 0.0, EPSILON14);
}

#[test]
fn test_log_base() {
    assert_approx!(log_base(8.0, 2.0), 3.0, EPSILON14);
    assert_approx!(log_base(100.0, 10.0), 2.0, EPSILON14);
    assert_approx!(log_base(1.0, 10.0), 0.0, EPSILON14);
}

#[test]
fn test_cbrt() {
    assert_approx!(cbrt(27.0), 3.0, EPSILON14);
    assert_approx!(cbrt(-8.0), -2.0, EPSILON14);
    assert_approx!(cbrt(0.0), 0.0, EPSILON14);
}

#[test]
fn test_mod_f64() {
    assert_approx!(mod_f64(5.0, 3.0), 2.0, EPSILON15);
    assert_approx!(mod_f64(-1.0, 3.0), 2.0, EPSILON15);
    assert_approx!(mod_f64(7.0, 3.0), 1.0, EPSILON15);
}

#[test]
fn test_is_zero() {
    assert!(is_zero(0.0));
    assert!(is_zero(1e-15));
    assert!(!is_zero(1e-13));
    assert!(!is_zero(1.0));
}

#[test]
fn test_cos_angle() {
    // cos(0) = 1
    assert_approx!(cos_angle(1.0, 1.0, 1.0), 1.0, EPSILON15);
    // cos(90°) = 0
    assert_approx!(cos_angle(0.0, 1.0, 1.0), 0.0, EPSILON15);
    // Clamped to [-1, 1]
    assert_approx!(cos_angle(2.0, 1.0, 1.0), 1.0, EPSILON15);
    assert_approx!(cos_angle(-2.0, 1.0, 1.0), -1.0, EPSILON15);
}
