//! Tests for `cesium_core::cylinder_geometry_library`.

use cesium_core::cylinder_geometry_library::compute_positions;

const EPSILON10: f64 = 1e-10;

#[test]
fn compute_positions_returns_correct_size_no_fill() {
    let positions = compute_positions(10.0, 5.0, 5.0, 8, false);
    // Without fill: 2 * slices * 3 (top + bottom rings)
    assert_eq!(positions.len(), 2 * 8 * 3);
}

#[test]
fn compute_positions_returns_correct_size_with_fill() {
    let positions = compute_positions(10.0, 5.0, 5.0, 8, true);
    // With fill: 2 * (2 * slices) * 3 (side + top/bottom caps)
    assert_eq!(positions.len(), 2 * (2 * 8) * 3);
}

#[test]
fn compute_positions_bottom_z_is_negative_half_length() {
    let positions = compute_positions(10.0, 5.0, 5.0, 4, false);
    // Bottom ring z values should be -5.0
    let bottom_z = positions[2]; // first position's z
    assert!((bottom_z - (-5.0)).abs() < EPSILON10);
}

#[test]
fn compute_positions_top_z_is_positive_half_length() {
    let positions = compute_positions(10.0, 5.0, 5.0, 4, false);
    // Top ring z values should be 5.0
    let top_offset = 4 * 3; // skip bottom ring
    let top_z = positions[top_offset + 2];
    assert!((top_z - 5.0).abs() < EPSILON10);
}

#[test]
fn compute_positions_radius_applied() {
    let positions = compute_positions(10.0, 3.0, 7.0, 4, false);
    // First bottom position x should be 7.0 * cos(0) = 7.0
    let bottom_x = positions[0];
    assert!((bottom_x - 7.0).abs() < EPSILON10);
}
