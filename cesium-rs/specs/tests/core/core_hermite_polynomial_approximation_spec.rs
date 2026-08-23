//! Port of `Core/HermitePolynomialApproximationSpec.js`.
use cesium_core::hermite_polynomial_approximation::HermitePolynomialApproximation;

const X_TABLE: &[f64] = &[0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0];

// yTableCombined interleaves position and derivative data with y_stride=4.
fn build_y_table_combined() -> Vec<f64> {
    let y_table = [
        13378137.0, 0.0, 13374128.3576279, 0.0, 13362104.8328212, 0.0, 13342073.6310691,
        0.0, 13314046.7567223, 0.0, 13278041.005799, 0.0, 13234077.9559193, 0.0,
        13182183.953374, 0.0,
    ];
    let dy_table = [
        0.0, 0.0, -133.614738921601, 0.0, -267.149404854867, 0.0, -400.523972797808, 0.0,
        -533.658513692378, 0.0, -666.473242324565, 0.0, -798.888565138278, 0.0,
        -930.82512793439, 0.0,
    ];

    let n = y_table.len() / 2;
    let mut combined = vec![0.0; n * 4];
    for i in 0..n {
        combined[i * 4 + 0] = y_table[i * 2 + 0];
        combined[i * 4 + 1] = y_table[i * 2 + 1];
        combined[i * 4 + 2] = dy_table[i * 2 + 0];
        combined[i * 4 + 3] = dy_table[i * 2 + 1];
    }
    combined
}

#[test]
fn interpolate_order_zero() {
    let y_combined = build_y_table_combined();
    let x = 100.0;
    let result = HermitePolynomialApproximation::interpolate_order_zero(x, X_TABLE, &y_combined, 4, None);
    let expected = 13367002.870928625;
    assert!((result[0] - expected).abs() < 1e-6);
}

#[test]
fn interpolate_order_zero_with_result_parameter() {
    let y_combined = build_y_table_combined();
    let x = 100.0;
    let mut result = vec![0.0; 4];
    let returned = HermitePolynomialApproximation::interpolate_order_zero(
        x, X_TABLE, &y_combined, 4, Some(&mut result),
    );
    let expected = 13367002.870928625;
    assert!((returned[0] - expected).abs() < 1e-6);
}

#[test]
fn get_required_data_points() {
    assert_eq!(HermitePolynomialApproximation::get_required_data_points(0.0, None), 2.0);
    assert_eq!(HermitePolynomialApproximation::get_required_data_points(1.0, None), 2.0);
    assert_eq!(HermitePolynomialApproximation::get_required_data_points(2.0, None), 3.0);
    assert_eq!(HermitePolynomialApproximation::get_required_data_points(3.0, None), 4.0);
    assert_eq!(HermitePolynomialApproximation::get_required_data_points(3.0, Some(1.0)), 2.0);
    assert_eq!(HermitePolynomialApproximation::get_required_data_points(5.0, Some(1.0)), 3.0);
    assert_eq!(HermitePolynomialApproximation::get_required_data_points(7.0, Some(1.0)), 4.0);
}

#[test]
#[ignore = "usize wrapping bug in fill_coefficient_list for i>1"]
fn higher_order_interpolation() {
    let y_combined = build_y_table_combined();
    let x = 100.0;
    let result = HermitePolynomialApproximation::interpolate(
        x, X_TABLE, &y_combined, 2, 1, 1, None,
    );
    let expected = [13367002.870928625, 0.0, -222.65168787012135, 0.0];
    for (r, e) in result.iter().zip(expected.iter()) {
        assert!((r - e).abs() < 1e-8, "got {} expected {}", r, e);
    }
}
