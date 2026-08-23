//! Port of `Core/QuadraticRealPolynomialSpec.js`.
use cesium_core::math::CesiumMath;
use cesium_core::quadratic_real_polynomial::QuadraticRealPolynomial;

#[test]
fn discriminant() {
    let d = QuadraticRealPolynomial::compute_discriminant(1.0, 2.0, 3.0);
    assert_eq!(d, -8.0);
}

#[test]
fn negative_b() {
    let roots = QuadraticRealPolynomial::compute_real_roots(2.0, -4.0, -6.0);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], -1.0);
    assert_eq!(roots[1], 3.0);
}

#[test]
fn positive_b() {
    let roots = QuadraticRealPolynomial::compute_real_roots(2.0, 4.0, -6.0);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], -3.0);
    assert_eq!(roots[1], 1.0);
}

#[test]
fn marginally_negative_radical_case() {
    let roots = QuadraticRealPolynomial::compute_real_roots(2.0, -3.999999999999999, 2.0);
    assert_eq!(roots.len(), 2);
    assert!((roots[0] - 1.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - 1.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn complex_roots() {
    let roots = QuadraticRealPolynomial::compute_real_roots(2.0, -4.0, 6.0);
    assert_eq!(roots.len(), 0);
}

#[test]
fn intractable_case() {
    let roots = QuadraticRealPolynomial::compute_real_roots(0.0, 0.0, -3.0);
    assert_eq!(roots.len(), 0);
}

#[test]
fn linear_case() {
    let roots = QuadraticRealPolynomial::compute_real_roots(0.0, 2.0, 8.0);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0], -4.0);
}

#[test]
fn second_order_monomial_case() {
    let roots = QuadraticRealPolynomial::compute_real_roots(3.0, 0.0, 0.0);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], 0.0);
    assert_eq!(roots[1], 0.0);
}

#[test]
fn parabolic_case_complex_roots() {
    let roots = QuadraticRealPolynomial::compute_real_roots(3.0, 0.0, 18.0);
    assert_eq!(roots.len(), 0);
}

#[test]
fn parabolic_case_real_roots() {
    let roots = QuadraticRealPolynomial::compute_real_roots(2.0, 0.0, -18.0);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], -3.0);
    assert_eq!(roots[1], 3.0);
}

#[test]
fn zero_and_negative_root_case() {
    let roots = QuadraticRealPolynomial::compute_real_roots(2.0, 6.0, 0.0);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], -3.0);
    assert_eq!(roots[1], 0.0);
}

#[test]
fn zero_and_positive_root_case() {
    let roots = QuadraticRealPolynomial::compute_real_roots(2.0, -6.0, 0.0);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], 0.0);
    assert_eq!(roots[1], 3.0);
}
