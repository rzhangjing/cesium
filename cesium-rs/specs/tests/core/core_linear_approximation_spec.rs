//! Port of `Core/LinearApproximationSpec.js`.
use cesium_core::linear_approximation;

#[test]
fn interpolate_order_zero_basic() {
    let x_table = [2.0, 4.0];
    let y_table = [2.0, 3.0, 4.0, 34.0];
    let mut result = vec![0.0; 2];
    linear_approximation::interpolate_order_zero(3.0, &x_table, &y_table, 2, &mut result);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], 3.0);
    assert_eq!(result[1], 18.5);
}

#[test]
fn interpolate_order_zero_second_case() {
    let x_table = [40.0, 120.0];
    let y_table = [20.0, 40.0, 60.0, 80.0, 90.0, 100.0];
    let mut result = vec![0.0; 3];
    linear_approximation::interpolate_order_zero(80.0, &x_table, &y_table, 3, &mut result);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 50.0);
    assert_eq!(result[1], 65.0);
    assert_eq!(result[2], 80.0);
}

#[test]
fn interpolate_order_zero_single_stride() {
    let x_table = [20.0, 30.0];
    let y_table = [10.0, 20.0, 30.0, 20.0, 30.0, 40.0, 20.0, 40.0, 60.0, 80.0, 90.0, 100.0];
    let mut result = vec![0.0; 1];
    linear_approximation::interpolate_order_zero(40.0, &x_table, &y_table, 1, &mut result);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], 30.0);
}

#[test]
fn get_required_data_points_returns_2() {
    assert_eq!(linear_approximation::get_required_data_points(1), 2);
}
