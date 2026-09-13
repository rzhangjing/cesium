//! Scene/AxisSpec.js → Rust integration tests
//!
//! Original: 6 it() → 6 A-class (axis conversion matrices)
//! Tests: y_up_to_z_up(1) + y_up_to_x_up(1) + z_up_to_x_up(1) +
//!        z_up_to_y_up(1) + x_up_to_y_up(1) + x_up_to_z_up(1)

use cesium_scene::axis::{
    Axis, X_UP_TO_Y_UP, X_UP_TO_Z_UP, Y_UP_TO_X_UP, Y_UP_TO_Z_UP, Z_UP_TO_X_UP, Z_UP_TO_Y_UP,
};
use glam::{DVec4, DMat4};

const EPSILON1: f64 = 1e-1;

fn convert_up_axis(up_axis: DVec4, transformation: DMat4, expected: DVec4) {
    let transformed = transformation * up_axis;
    let len = transformed.length();
    let normalized = if len > 0.0 { transformed / len } else { transformed };
    assert!(
        (normalized.x - expected.x).abs() < EPSILON1
            && (normalized.y - expected.y).abs() < EPSILON1
            && (normalized.z - expected.z).abs() < EPSILON1
            && (normalized.w - expected.w).abs() < EPSILON1,
        "Expected {:?}, got {:?}",
        expected,
        normalized
    );
}

#[test]
fn test_convert_y_up_to_z_up() {
    convert_up_axis(DVec4::Y, Y_UP_TO_Z_UP, DVec4::Z);
}

#[test]
fn test_convert_y_up_to_x_up() {
    convert_up_axis(DVec4::Y, Y_UP_TO_X_UP, DVec4::X);
}

#[test]
fn test_convert_z_up_to_x_up() {
    convert_up_axis(DVec4::Z, Z_UP_TO_X_UP, DVec4::X);
}

#[test]
fn test_convert_z_up_to_y_up() {
    convert_up_axis(DVec4::Z, Z_UP_TO_Y_UP, DVec4::Y);
}

#[test]
fn test_convert_x_up_to_y_up() {
    convert_up_axis(DVec4::X, X_UP_TO_Y_UP, DVec4::Y);
}

#[test]
fn test_convert_x_up_to_z_up() {
    convert_up_axis(DVec4::X, X_UP_TO_Z_UP, DVec4::Z);
}

#[test]
fn test_axis_from_name() {
    assert_eq!(Axis::from_name("X"), Some(Axis::X));
    assert_eq!(Axis::from_name("Y"), Some(Axis::Y));
    assert_eq!(Axis::from_name("Z"), Some(Axis::Z));
    assert_eq!(Axis::from_name("W"), None);
}
