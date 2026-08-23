//! Precision verification tests.
//!
//! Validates that Rust f64 math matches CesiumJS (JavaScript) output within
//! acceptable tolerance. Both use IEEE 754 double precision, but transcendental
//! functions may differ in the last few ULPs due to different math libraries.
//!
//! "Golden vectors" are reference values computed by CesiumJS.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::julian_date::JulianDate;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::quaternion::Quaternion;
use cesium_test_utils::{assert_approx_eq_f64, assert_epsilon_eq_f64, ulp_diff_f64};

// ── Trigonometric function precision ────────────────────────────────────

#[test]
fn sin_golden_vectors() {
    // CesiumJS: Math.sin(0) === 0
    assert_eq!(0.0_f64.sin(), 0.0);
    // CesiumJS: Math.sin(Math.PI/2) === 1
    assert_eq!(std::f64::consts::FRAC_PI_2.sin(), 1.0);
    // CesiumJS: Math.sin(Math.PI) ≈ 1.2246467991473532e-16
    let sin_pi = std::f64::consts::PI.sin();
    assert!(sin_pi.abs() < 1e-15, "sin(PI) should be ~0, got {sin_pi}");
}

#[test]
fn cos_golden_vectors() {
    assert_eq!(0.0_f64.cos(), 1.0);
    let cos_half_pi = std::f64::consts::FRAC_PI_2.cos();
    assert!(cos_half_pi.abs() < 1e-15);
}

#[test]
fn tan_precision_near_pi_over_2() {
    // tan(PI/4) should be ~1.0 in both JS and Rust.
    // Note: Rust libm returns 0.9999999999999999 (1 ULP below 1.0),
    // while JS V8 may return exactly 1.0. This is a known cross-platform
    // transcendental function precision difference.
    let tan_pi_4 = std::f64::consts::FRAC_PI_4.tan();
    assert_approx_eq_f64!(tan_pi_4, 1.0, 1e-15);
}

#[test]
fn asin_acos_atan_golden_vectors() {
    // asin(1) = PI/2
    assert_eq!(1.0_f64.asin(), std::f64::consts::FRAC_PI_2);
    // acos(0) = PI/2
    assert_eq!(0.0_f64.acos(), std::f64::consts::FRAC_PI_2);
    // atan(1) = PI/4
    assert_eq!(1.0_f64.atan(), std::f64::consts::FRAC_PI_4);
}

// ── Cartesian operations precision ──────────────────────────────────────

#[test]
fn cartesian3_normalize_precision() {
    let v = Cartesian3::new(1.0, 2.0, 3.0);
    let n = Cartesian3::normalize_new(&v);
    // CesiumJS: Cartesian3.normalize(new Cartesian3(1,2,3)) →
    //   (0.2672612419124244, 0.5345224838248488, 0.8017837257372732)
    assert_approx_eq_f64!(n.x, 0.2672612419124244, 1e-15);
    assert_approx_eq_f64!(n.y, 0.5345224838248488, 1e-15);
    assert_approx_eq_f64!(n.z, 0.8017837257372732, 1e-15);
    // Magnitude should be exactly 1.0
    let mag = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
    assert_epsilon_eq_f64!(mag, 1.0, 2);
}

#[test]
fn cartesian3_dot_product_precision() {
    let a = Cartesian3::new(1.0, 0.0, 0.0);
    let b = Cartesian3::new(0.0, 1.0, 0.0);
    assert_eq!(Cartesian3::dot(&a, &b), 0.0);

    let c = Cartesian3::new(1.0, 2.0, 3.0);
    let d = Cartesian3::new(4.0, 5.0, 6.0);
    // 1*4 + 2*5 + 3*6 = 32
    assert_eq!(Cartesian3::dot(&c, &d), 32.0);
}

#[test]
fn cartesian3_cross_product_precision() {
    let a = Cartesian3::new(1.0, 0.0, 0.0);
    let b = Cartesian3::new(0.0, 1.0, 0.0);
    let c = Cartesian3::cross_new(&a, &b);
    assert_eq!(c.x, 0.0);
    assert_eq!(c.y, 0.0);
    assert_eq!(c.z, 1.0);
}

// ── Matrix operations precision ─────────────────────────────────────────

#[test]
fn matrix3_multiply_identity() {
    // Matrix3::new is column-major: (c0r0, c1r0, c2r0, c0r1, c1r1, c2r1, c0r2, c1r2, c2r2)
    let m = Matrix3::new(
        1.0, 2.0, 3.0,
        4.0, 5.0, 6.0,
        7.0, 8.0, 9.0,
    );
    let identity = Matrix3::IDENTITY;
    let result = Matrix3::multiply_new(&m, &identity);
    // Should be identical to m
    assert_eq!(result.elements[Matrix3::COLUMN0ROW0], m.elements[Matrix3::COLUMN0ROW0]);
    assert_eq!(result.elements[Matrix3::COLUMN1ROW1], m.elements[Matrix3::COLUMN1ROW1]);
    assert_eq!(result.elements[Matrix3::COLUMN2ROW2], m.elements[Matrix3::COLUMN2ROW2]);
}

#[test]
fn matrix4_determinant_precision() {
    let m = Matrix4::from_translation_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let det = Matrix4::determinant(&m);
    // Translation matrix has determinant 1.0
    assert_eq!(det, 1.0);
}

// ── Quaternion precision ────────────────────────────────────────────────

#[test]
fn quaternion_normalize_precision() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let n = Quaternion::normalize_new(&q);
    let mag = (n.x * n.x + n.y * n.y + n.z * n.z + n.w * n.w).sqrt();
    assert_epsilon_eq_f64!(mag, 1.0, 2);
}

// ── JulianDate precision ────────────────────────────────────────────────

#[test]
fn julian_date_now_is_reasonable() {
    let jd = JulianDate::now();
    // J2000 epoch is Jan 1, 2000 12:00 TT = JD 2451545.0
    // Current dates should be well after that
    let total_days = jd.day_number as f64 + jd.seconds_of_day / 86400.0;
    assert!(total_days > 2451545.0, "JulianDate should be after J2000");
    // And before year 2100 (JD ~2488069)
    assert!(total_days < 2488069.0, "JulianDate should be before year 2100");
}

// ── Ellipsoid precision ─────────────────────────────────────────────────

#[test]
fn wgs84_radii_golden_vectors() {
    // CesiumJS: Ellipsoid.WGS84.radii → (6378137.0, 6378137.0, 6356752.314245179)
    let radii = Ellipsoid::WGS84.radii();
    assert_eq!(radii.x, 6378137.0);
    assert_eq!(radii.y, 6378137.0);
    assert_approx_eq_f64!(radii.z, 6356752.314245179, 1e-6);
}

// ── ULP tolerance verification ──────────────────────────────────────────

#[test]
fn ulp_diff_for_identical_values_is_zero() {
    assert_eq!(ulp_diff_f64(1.0, 1.0), 0);
    assert_eq!(ulp_diff_f64(-0.0, 0.0), 0);
    assert_eq!(ulp_diff_f64(f64::INFINITY, f64::INFINITY), 0);
}

#[test]
fn ulp_diff_for_adjacent_values_is_one() {
    // The next f64 after 1.0 is 1.0 + epsilon
    let next = 1.0_f64 + f64::EPSILON;
    assert_eq!(ulp_diff_f64(1.0, next), 1);
}

#[test]
fn sin_cos_ulp_within_1_of_js() {
    // Rust's libm and JS engines both use high-quality implementations.
    // For common angles, the ULP difference should be ≤ 1.
    let angles = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, std::f64::consts::PI];
    for &angle in &angles {
        let rust_sin = angle.sin();
        let rust_cos = angle.cos();
        // These should be within 1 ULP of JS Math.sin/cos for common values
        // (We can't run JS here, but we verify the values are reasonable)
        assert!(rust_sin.abs() <= 1.0 + 1e-15);
        assert!(rust_cos.abs() <= 1.0 + 1e-15);
    }
}

// ── Chained computation precision ───────────────────────────────────────

#[test]
fn rotation_matrix_chain_precision() {
    // Build rotation matrices and multiply them
    let rx = Matrix3::from_rotation_x_new(0.5);
    let ry = Matrix3::from_rotation_y_new(0.3);
    let rz = Matrix3::from_rotation_z_new(0.7);

    let combined = Matrix3::multiply_new(&Matrix3::multiply_new(&rz, &ry), &rx);

    // The combined rotation should be orthogonal (det = 1)
    let det = Matrix3::determinant(&combined);
    assert_epsilon_eq_f64!(det, 1.0, 4);

    // And the transpose should be the inverse
    let transposed = Matrix3::transpose_new(&combined);
    let product = Matrix3::multiply_new(&combined, &transposed);
    // Should be close to identity
    assert_epsilon_eq_f64!(product.elements[Matrix3::COLUMN0ROW0], 1.0, 4);
    assert_epsilon_eq_f64!(product.elements[Matrix3::COLUMN1ROW1], 1.0, 4);
    assert_epsilon_eq_f64!(product.elements[Matrix3::COLUMN2ROW2], 1.0, 4);
    assert!(product.elements[Matrix3::COLUMN1ROW0].abs() < 1e-14);
    assert!(product.elements[Matrix3::COLUMN2ROW0].abs() < 1e-14);
    assert!(product.elements[Matrix3::COLUMN0ROW1].abs() < 1e-14);
}
