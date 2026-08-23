//! Port of `Core/CubicRealPolynomialSpec.js`.
use cesium_core::cubic_real_polynomial::CubicRealPolynomial;
use cesium_core::math::CesiumMath;

#[test]
fn discriminant() {
    let a = 3.0;
    let b = 2.0;
    let c = 1.0;
    let d = 1.0;
    let expected = b * b * c * c - 4.0 * a * c * c * c - 4.0 * b * b * b * d
        - 27.0 * a * a * d * d + 18.0 * a * b * c * d;
    let actual = CubicRealPolynomial::compute_discriminant(a, b, c, d);
    assert!((actual - expected).abs() < CesiumMath::EPSILON14);
}

#[test]
fn three_repeated_roots() {
    let roots = CubicRealPolynomial::compute_real_roots(2.0, -12.0, 24.0, -16.0);
    assert_eq!(roots.len(), 3);
    assert!((roots[0] - 2.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - 2.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 2.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn one_unique_and_two_repeated_roots() {
    let roots = CubicRealPolynomial::compute_real_roots(2.0, 2.0, -2.0, -2.0);
    assert_eq!(roots.len(), 3);
    assert!((roots[0] - (-1.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - (-1.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 1.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn three_unique_roots() {
    let roots = CubicRealPolynomial::compute_real_roots(2.0, 6.0, -26.0, -30.0);
    assert_eq!(roots.len(), 3);
    assert!((roots[0] - (-5.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - (-1.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 3.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn complex_roots() {
    let roots = CubicRealPolynomial::compute_real_roots(2.0, -6.0, 10.0, -6.0);
    assert_eq!(roots.len(), 1);
    assert!((roots[0] - 1.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn quadratic_case() {
    let roots = CubicRealPolynomial::compute_real_roots(0.0, 2.0, -4.0, -6.0);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], -1.0);
    assert_eq!(roots[1], 3.0);
}

#[test]
fn deflated_cases() {
    let roots = CubicRealPolynomial::compute_real_roots(1.0, 0.0, 1.0, 2.0);
    assert_eq!(roots.len(), 1);
    assert!((roots[0] - (-1.0)).abs() < CesiumMath::EPSILON14);

    let roots = CubicRealPolynomial::compute_real_roots(1.0, 0.0, 0.0, -8.0);
    assert_eq!(roots.len(), 3);
    assert!((roots[0] - 2.0).abs() < CesiumMath::EPSILON14);

    let roots = CubicRealPolynomial::compute_real_roots(1.0, 0.0, -1.0, 0.0);
    assert_eq!(roots.len(), 3);
    assert_eq!(roots[0], -1.0);
    assert_eq!(roots[1], 0.0);
    assert_eq!(roots[2], 1.0);

    let roots = CubicRealPolynomial::compute_real_roots(1.0, 1.0, 0.0, 0.0);
    assert_eq!(roots.len(), 3);
    assert_eq!(roots[0], -1.0);
    assert_eq!(roots[1], 0.0);
    assert_eq!(roots[2], 0.0);

    let roots = CubicRealPolynomial::compute_real_roots(1.0, -1.0, 0.0, 0.0);
    assert_eq!(roots.len(), 3);
    assert_eq!(roots[0], 0.0);
    assert_eq!(roots[1], 0.0);
    assert_eq!(roots[2], 1.0);

    let roots = CubicRealPolynomial::compute_real_roots(1.0, 1.0, 1.0, 0.0);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0], 0.0);
}
