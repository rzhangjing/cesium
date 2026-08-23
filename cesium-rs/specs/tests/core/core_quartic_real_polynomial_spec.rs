use cesium_core::math::CesiumMath;
use cesium_core::quartic_real_polynomial::QuarticRealPolynomial;

#[test]
fn discriminant() {
    let a = 1.0;
    let b = 2.0;
    let c = 3.0;
    let d = 4.0;
    let e = 5.0;

    let a2 = a * a;
    let a3 = a2 * a;
    let b2 = b * b;
    let b3 = b2 * b;
    let c2 = c * c;
    let c3 = c2 * c;
    let d2 = d * d;
    let d3 = d2 * d;
    let e2 = e * e;
    let e3 = e2 * e;

    let expected = b2 * c2 * d2
        - 4.0 * b3 * d3
        - 4.0 * a * c3 * d2
        + 18.0 * a * b * c * d3
        - 27.0 * a2 * d2 * d2
        + 256.0 * a3 * e3
        + e * (18.0 * b3 * c * d
            - 4.0 * b2 * c3
            + 16.0 * a * c2 * c2
            - 80.0 * a * b * c2 * d
            - 6.0 * a * b2 * d2
            + 144.0 * a2 * c * d2)
        + e2 * (144.0 * a * b2 * c
            - 27.0 * b2 * b2
            - 128.0 * a2 * c2
            - 192.0 * a2 * b * d);

    let actual = QuarticRealPolynomial::compute_discriminant(a, b, c, d, e);
    assert_eq!(actual, expected);
}

#[test]
fn four_repeated_roots() {
    let roots = QuarticRealPolynomial::compute_real_roots(2.0, -16.0, 48.0, -64.0, 32.0);
    assert_eq!(roots.len(), 4);
    assert!((roots[0] - 2.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - 2.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 2.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[3] - 2.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn two_pairs_of_repeated_roots() {
    let roots = QuarticRealPolynomial::compute_real_roots(2.0, 0.0, -4.0, 0.0, 2.0);
    assert_eq!(roots.len(), 4);
    assert!((roots[0] - (-1.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - (-1.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 1.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[3] - 1.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn one_pair_of_repeated_roots() {
    let roots = QuarticRealPolynomial::compute_real_roots(2.0, -8.0, 16.0, -16.0, 6.0);
    assert_eq!(roots.len(), 2);
    assert!((roots[0] - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((roots[1] - 1.0).abs() < CesiumMath::EPSILON14);
}

#[test]
fn two_unique_and_one_pair_of_repeated_roots() {
    let roots = QuarticRealPolynomial::compute_real_roots(2.0, 8.0, -6.0, -20.0, 16.0);
    assert_eq!(roots.len(), 4);
    assert!((roots[0] - (-4.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - (-2.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 1.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[3] - 1.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn four_unique_roots() {
    let roots = QuarticRealPolynomial::compute_real_roots(2.0, 4.0, -26.0, -28.0, 48.0);
    assert_eq!(roots.len(), 4);
    assert!((roots[0] - (-4.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - (-2.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 1.0).abs() < CesiumMath::EPSILON15);
    assert!((roots[3] - 3.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn complex_roots() {
    let roots = QuarticRealPolynomial::compute_real_roots(3.0, -8.0, 14.0, -8.0, 3.0);
    assert_eq!(roots.len(), 0);
}

#[test]
fn cubic_case() {
    let roots = QuarticRealPolynomial::compute_real_roots(0.0, 2.0, 6.0, -26.0, -30.0);
    assert_eq!(roots.len(), 3);
    assert!((roots[0] - (-5.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[1] - (-1.0)).abs() < CesiumMath::EPSILON15);
    assert!((roots[2] - 3.0).abs() < CesiumMath::EPSILON15);
}

#[test]
fn stability_1() {
    let a = 1.0;
    let b = -27121.309311434146;
    let c = 0.0;
    let d = -26760.571078686513;
    let e = -1.0;

    let expected = [-0.000037368410630733706, 27121.3093478151];
    let actual = QuarticRealPolynomial::compute_real_roots(a, b, c, d, e);
    assert_eq!(actual.len(), expected.len());
    assert!((actual[0] - expected[0]).abs() < CesiumMath::EPSILON12);
    assert!((actual[1] - expected[1]).abs() < CesiumMath::EPSILON12);
}

#[test]
fn stability_2() {
    let a = -1.0;
    let b = -26959.661445199898;
    let c = 0.0;
    let d = -26675.609408851604;
    let e = 1.0;

    let expected = [-26959.661481901538, 0.000037487427107407711];
    let actual = QuarticRealPolynomial::compute_real_roots(a, b, c, d, e);
    assert_eq!(actual.len(), expected.len());
    assert!((actual[0] - expected[0]).abs() < CesiumMath::EPSILON11);
    assert!((actual[1] - expected[1]).abs() < CesiumMath::EPSILON11);
}

#[test]
fn stability_3() {
    let a = -1.0;
    let b = 20607.270539372261;
    let c = 0.0;
    let d = 20333.159863900513;
    let e = 1.0;

    let expected = [-0.000049180747737409547, 20607.270587253341];
    let actual = QuarticRealPolynomial::compute_real_roots(a, b, c, d, e);
    assert_eq!(actual.len(), expected.len());
    assert!((actual[0] - expected[0]).abs() < CesiumMath::EPSILON11);
    assert!((actual[1] - expected[1]).abs() < CesiumMath::EPSILON11);
}
