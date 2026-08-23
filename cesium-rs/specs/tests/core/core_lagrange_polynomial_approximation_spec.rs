//! Port of `Core/LagrangePolynomialApproximationSpec.js`.
use cesium_core::lagrange_polynomial_approximation::LagrangePolynomialApproximation;

const X_TABLE: &[f64] = &[0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0];
const Y_TABLE: &[f64] = &[
    13378137.0, 0.0, 0.0,
    13374128.3576279, 327475.593690065, 0.0,
    13362104.8328212, 654754.936954423, 0.0,
    13342073.6310691, 981641.896976832, 0.0,
    13314046.7567223, 1307940.57608951, 0.0,
    13278041.005799, 1633455.42917117, 0.0,
    13234077.9559193, 1957991.38083385, 0.0,
    13182183.953374, 2281353.94232816, 0.0,
];

#[test]
fn interpolate_order_zero() {
    let x = 100.0;
    let result = LagrangePolynomialApproximation::interpolate_order_zero(x, X_TABLE, Y_TABLE, 3, None);
    assert!((result[0] - 13367002.870928623).abs() < 1e-7);
    assert!((result[1] - 545695.7388100647).abs() < 1e-7);
    assert!((result[2] - 0.0).abs() < 1e-15);
}

#[test]
fn interpolate_order_zero_with_result_parameter() {
    let x = 100.0;
    let mut result = vec![0.0; 3];
    let returned = LagrangePolynomialApproximation::interpolate_order_zero(
        x, X_TABLE, Y_TABLE, 3, Some(&mut result),
    );
    assert!((returned[0] - 13367002.870928623).abs() < 1e-7);
    assert!((returned[1] - 545695.7388100647).abs() < 1e-7);
}

#[test]
fn get_required_data_points() {
    assert_eq!(LagrangePolynomialApproximation::get_required_data_points(0.0), 2.0);
    assert_eq!(LagrangePolynomialApproximation::get_required_data_points(1.0), 2.0);
    assert_eq!(LagrangePolynomialApproximation::get_required_data_points(2.0), 3.0);
    assert_eq!(LagrangePolynomialApproximation::get_required_data_points(3.0), 4.0);
}
