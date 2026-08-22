//! Mirrors packages/engine/Specs/Core/QuaternionSpec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::heading_pitch_roll::HeadingPitchRoll;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::quaternion::Quaternion;
use cesium_test_utils::assert_approx_eq_f64;

// --- constructor ---

#[test]
fn default_constructor_creates_zero() {
    let q = Quaternion::default();
    assert_eq!(q.x, 0.0);
    assert_eq!(q.y, 0.0);
    assert_eq!(q.z, 0.0);
    assert_eq!(q.w, 0.0);
}

#[test]
fn constructor_with_all_values() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(q.x, 1.0);
    assert_eq!(q.y, 2.0);
    assert_eq!(q.z, 3.0);
    assert_eq!(q.w, 4.0);
}

// --- fromAxisAngle ---

#[test]
fn from_axis_angle_works() {
    let axis = Cartesian3::new(0.0, 0.0, 1.0);
    let angle = CesiumMath::PI_OVER_TWO;
    let s = (angle / 2.0).sin();
    let c = (angle / 2.0).cos();
    let a = Cartesian3::multiply_by_scalar_new(&axis, s);
    let expected = Quaternion::new(a.x, a.y, a.z, c);
    let result = Quaternion::from_axis_angle_new(&axis, angle);
    assert!(Quaternion::equals_epsilon(&result, &expected, CesiumMath::EPSILON15));
}

#[test]
fn from_axis_angle_with_result() {
    let axis = Cartesian3::new(0.0, 0.0, 1.0);
    let angle = CesiumMath::PI_OVER_TWO;
    let s = (angle / 2.0).sin();
    let c = (angle / 2.0).cos();
    let a = Cartesian3::multiply_by_scalar_new(&axis, s);
    let expected = Quaternion::new(a.x, a.y, a.z, c);
    let mut result = Quaternion::default();
    Quaternion::from_axis_angle(&axis, angle, &mut result);
    assert!(Quaternion::equals_epsilon(&result, &expected, CesiumMath::EPSILON15));
}

// --- fromRotationMatrix ---

#[test]
fn from_rotation_matrix_m22_max() {
    let q = Quaternion::from_axis_angle_new(&Cartesian3::negate_new(&Cartesian3::UNIT_Z), std::f64::consts::PI);
    let rotation = Matrix3::new(-1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0);
    let result = Quaternion::from_rotation_matrix_new(&rotation);
    assert!(Quaternion::equals_epsilon(&result, &q, CesiumMath::EPSILON15));
}

#[test]
fn from_rotation_matrix_m11_max() {
    let q = Quaternion::from_axis_angle_new(&Cartesian3::negate_new(&Cartesian3::UNIT_Y), std::f64::consts::PI);
    let rotation = Matrix3::new(-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0);
    let result = Quaternion::from_rotation_matrix_new(&rotation);
    assert!(Quaternion::equals_epsilon(&result, &q, CesiumMath::EPSILON15));
}

#[test]
fn from_rotation_matrix_m00_max() {
    let q = Quaternion::from_axis_angle_new(&Cartesian3::negate_new(&Cartesian3::UNIT_X), std::f64::consts::PI);
    let rotation = Matrix3::new(1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0);
    let result = Quaternion::from_rotation_matrix_new(&rotation);
    assert!(Quaternion::equals_epsilon(&result, &q, CesiumMath::EPSILON15));
}

#[test]
fn from_rotation_matrix_trace_positive() {
    let rotation = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    let q = Quaternion::new(0.0, 0.0, 0.0, 1.0);
    let result = Quaternion::from_rotation_matrix_new(&rotation);
    assert!(Quaternion::equals_epsilon(&result, &q, CesiumMath::EPSILON15));
}

#[test]
fn from_rotation_matrix_view_matrix() {
    let direction = Cartesian3::new(-0.2349326833984488, 0.8513513009480378, 0.46904967396353314);
    let up = Cartesian3::new(0.12477198625717335, -0.4521499177166376, 0.8831717858696695);
    let right = Cartesian3::new(0.9639702203483635, 0.26601017702986895, 6.456422901079747e-10);
    let matrix = Matrix3::new(
        right.x, up.x, -direction.x,
        right.y, up.y, -direction.y,
        right.z, up.z, -direction.z,
    );
    let quaternion = Quaternion::from_rotation_matrix_new(&matrix);
    let roundtrip = Matrix3::from_quaternion_new(&quaternion);
    assert!(Matrix3::equals_epsilon(&roundtrip, &matrix, CesiumMath::EPSILON12));
}

// --- fromHeadingPitchRoll ---

#[test]
fn from_heading_pitch_roll_heading_only() {
    let angle = CesiumMath::to_radians(20.0);
    let hpr = HeadingPitchRoll::new(angle, 0.0, 0.0);
    let quaternion = Quaternion::from_heading_pitch_roll_new(&hpr);
    let result_matrix = Matrix3::from_quaternion_new(&quaternion);
    let expected = Matrix3::from_rotation_z_new(-angle);
    assert!(Matrix3::equals_epsilon(&result_matrix, &expected, CesiumMath::EPSILON11));
}

#[test]
fn from_heading_pitch_roll_pitch_only() {
    let angle = CesiumMath::to_radians(20.0);
    let hpr = HeadingPitchRoll::new(0.0, angle, 0.0);
    let quaternion = Quaternion::from_heading_pitch_roll_new(&hpr);
    let result_matrix = Matrix3::from_quaternion_new(&quaternion);
    let expected = Matrix3::from_rotation_y_new(-angle);
    assert!(Matrix3::equals_epsilon(&result_matrix, &expected, CesiumMath::EPSILON11));
}

#[test]
fn from_heading_pitch_roll_roll_only() {
    let angle = CesiumMath::to_radians(20.0);
    let hpr = HeadingPitchRoll::new(0.0, 0.0, angle);
    let quaternion = Quaternion::from_heading_pitch_roll_new(&hpr);
    let result_matrix = Matrix3::from_quaternion_new(&quaternion);
    let expected = Matrix3::from_rotation_x_new(angle);
    assert!(Matrix3::equals_epsilon(&result_matrix, &expected, CesiumMath::EPSILON11));
}

#[test]
fn from_heading_pitch_roll_all_angles() {
    let angle = CesiumMath::to_radians(20.0);
    let hpr = HeadingPitchRoll::new(angle, angle, angle);
    let quaternion = Quaternion::from_heading_pitch_roll_new(&hpr);
    let mut expected = Matrix3::from_rotation_x_new(angle);
    let ry = Matrix3::from_rotation_y_new(-angle);
    let tmp = expected;
    Matrix3::multiply(&ry, &tmp, &mut expected);
    let rz = Matrix3::from_rotation_z_new(-angle);
    let tmp2 = expected;
    Matrix3::multiply(&rz, &tmp2, &mut expected);
    let result_matrix = Matrix3::from_quaternion_new(&quaternion);
    assert!(Matrix3::equals_epsilon(&result_matrix, &expected, CesiumMath::EPSILON11));
}

// --- clone ---

#[test]
fn clone_works() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let result = Quaternion::clone_new(&q);
    assert_eq!(result, q);
}

// --- conjugate ---

#[test]
fn conjugate_works() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let expected = Quaternion::new(-1.0, -2.0, -3.0, 4.0);
    let result = Quaternion::conjugate_new(&q);
    assert_eq!(result, expected);
}

// --- magnitudeSquared / magnitude ---

#[test]
fn magnitude_squared_works() {
    let q = Quaternion::new(2.0, 3.0, 4.0, 5.0);
    let expected = 2.0 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0 + 5.0 * 5.0;
    assert_eq!(Quaternion::magnitude_squared(&q), expected);
}

#[test]
fn magnitude_works() {
    let q = Quaternion::new(2.0, 3.0, 4.0, 5.0);
    let expected: f64 = (2.0_f64 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0 + 5.0 * 5.0).sqrt();
    assert_eq!(Quaternion::magnitude(&q), expected);
}

// --- normalize ---

#[test]
fn normalize_works() {
    let q = Quaternion::new(2.0, 0.0, 0.0, 0.0);
    let expected = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let result = Quaternion::normalize_new(&q);
    assert_eq!(result, expected);
}

// --- inverse ---

#[test]
fn inverse_works() {
    let q = Quaternion::new(2.0, 3.0, 4.0, 5.0);
    let mag_sq = Quaternion::magnitude_squared(&q);
    let expected = Quaternion::new(-2.0 / mag_sq, -3.0 / mag_sq, -4.0 / mag_sq, 5.0 / mag_sq);
    let result = Quaternion::inverse_new(&q);
    assert_eq!(result, expected);
}

// --- dot ---

#[test]
fn dot_works() {
    let left = Quaternion::new(2.0, 3.0, 6.0, 8.0);
    let right = Quaternion::new(4.0, 5.0, 7.0, 9.0);
    assert_eq!(Quaternion::dot(&left, &right), 137.0);
}

// --- multiply ---

#[test]
fn multiply_works() {
    let left = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let right = Quaternion::new(8.0, 7.0, 6.0, 5.0);
    let expected = Quaternion::new(28.0, 56.0, 30.0, -20.0);
    let result = Quaternion::multiply_new(&left, &right);
    assert_eq!(result, expected);
}

// --- add / subtract ---

#[test]
fn add_works() {
    let left = Quaternion::new(2.0, 3.0, 6.0, 8.0);
    let right = Quaternion::new(4.0, 5.0, 7.0, 9.0);
    let expected = Quaternion::new(6.0, 8.0, 13.0, 17.0);
    let result = Quaternion::add_new(&left, &right);
    assert_eq!(result, expected);
}

#[test]
fn subtract_works() {
    let left = Quaternion::new(2.0, 3.0, 4.0, 8.0);
    let right = Quaternion::new(1.0, 5.0, 7.0, 9.0);
    let expected = Quaternion::new(1.0, -2.0, -3.0, -1.0);
    let result = Quaternion::subtract_new(&left, &right);
    assert_eq!(result, expected);
}

// --- multiplyByScalar / divideByScalar ---

#[test]
fn multiply_by_scalar_works() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let expected = Quaternion::new(2.0, 4.0, 6.0, 8.0);
    let result = Quaternion::multiply_by_scalar_new(&q, 2.0);
    assert_eq!(result, expected);
}

#[test]
fn divide_by_scalar_works() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let expected = Quaternion::new(0.5, 1.0, 1.5, 2.0);
    let result = Quaternion::divide_by_scalar_new(&q, 2.0);
    assert_eq!(result, expected);
}

// --- computeAxis / computeAngle ---

#[test]
fn compute_axis_works() {
    let angle = std::f64::consts::PI / 3.0;
    let cos = (angle / 2.0).cos();
    let sin = (angle / 2.0).sin();
    let expected = Cartesian3::normalize_new(&Cartesian3::new(2.0, 3.0, 6.0));
    let q = Quaternion::new(sin * expected.x, sin * expected.y, sin * expected.z, cos);
    let result = Quaternion::compute_axis_new(&q);
    assert!(result.equals_epsilon_method(&expected, None, Some(CesiumMath::EPSILON15)));
}

#[test]
fn compute_axis_returns_unit_x_when_w_is_one() {
    let expected = Cartesian3::new(1.0, 0.0, 0.0);
    let q = Quaternion::new(4.0, 2.0, 3.0, 1.0);
    let result = Quaternion::compute_axis_new(&q);
    assert_eq!(result, expected);
}

#[test]
fn compute_angle_works() {
    let angle = std::f64::consts::PI / 3.0;
    let cos = (angle / 2.0).cos();
    let sin = (angle / 2.0).sin();
    let axis = Cartesian3::normalize_new(&Cartesian3::new(2.0, 3.0, 6.0));
    let q = Quaternion::new(sin * axis.x, sin * axis.y, sin * axis.z, cos);
    let result = Quaternion::compute_angle(&q);
    assert_approx_eq_f64!(result, angle, CesiumMath::EPSILON15);
}

// --- negate ---

#[test]
fn negate_works() {
    let q = Quaternion::new(1.0, -2.0, -5.0, 4.0);
    let expected = Quaternion::new(-1.0, 2.0, 5.0, -4.0);
    let result = Quaternion::negate_new(&q);
    assert_eq!(result, expected);
}

// --- lerp ---

#[test]
fn lerp_works() {
    let start = Quaternion::new(4.0, 8.0, 10.0, 20.0);
    let end = Quaternion::new(8.0, 20.0, 20.0, 30.0);
    let t = 0.25;
    let expected = Quaternion::new(5.0, 11.0, 12.5, 22.5);
    let result = Quaternion::lerp_new(&start, &end, t);
    assert_eq!(result, expected);
}

#[test]
fn lerp_extrapolate_forward() {
    let start = Quaternion::new(4.0, 8.0, 10.0, 20.0);
    let end = Quaternion::new(8.0, 20.0, 20.0, 30.0);
    let t = 2.0;
    let expected = Quaternion::new(12.0, 32.0, 30.0, 40.0);
    let result = Quaternion::lerp_new(&start, &end, t);
    assert_eq!(result, expected);
}

#[test]
fn lerp_extrapolate_backward() {
    let start = Quaternion::new(4.0, 8.0, 10.0, 20.0);
    let end = Quaternion::new(8.0, 20.0, 20.0, 30.0);
    let t = -1.0;
    let expected = Quaternion::new(0.0, -4.0, 0.0, 10.0);
    let result = Quaternion::lerp_new(&start, &end, t);
    assert_eq!(result, expected);
}

// --- slerp ---

#[test]
fn slerp_works() {
    let start = Quaternion::normalize_new(&Quaternion::new(0.0, 0.0, 0.0, 1.0));
    let end = Quaternion::new(0.0, 0.0, CesiumMath::PI_OVER_FOUR.sin(), CesiumMath::PI_OVER_FOUR.cos());
    let expected = Quaternion::new(0.0, 0.0, (std::f64::consts::PI / 8.0).sin(), (std::f64::consts::PI / 8.0).cos());
    let result = Quaternion::slerp_new(&start, &end, 0.5);
    assert!(Quaternion::equals_epsilon(&result, &expected, CesiumMath::EPSILON15));
}

#[test]
fn slerp_obtuse_angles() {
    let start = Quaternion::normalize_new(&Quaternion::new(0.0, 0.0, 0.0, -1.0));
    let end = Quaternion::new(0.0, 0.0, CesiumMath::PI_OVER_FOUR.sin(), CesiumMath::PI_OVER_FOUR.cos());
    let expected = Quaternion::new(0.0, 0.0, -(std::f64::consts::PI / 8.0).sin(), -(std::f64::consts::PI / 8.0).cos());
    let result = Quaternion::slerp_new(&start, &end, 0.5);
    assert!(Quaternion::equals_epsilon(&result, &expected, CesiumMath::EPSILON15));
}

#[test]
fn slerp_uses_lerp_when_close() {
    let start = Quaternion::new(0.0, 0.0, 0.0, 1.0);
    let end = Quaternion::new(1.0, 2.0, 3.0, 1.0);
    let expected = Quaternion::new(0.5, 1.0, 1.5, 1.0);
    let mut result = Quaternion::default();
    Quaternion::slerp(&start, &end, 0.5, &mut result);
    assert_eq!(result, expected);
}

// --- log / exp ---

#[test]
fn log_works() {
    let axis = Cartesian3::normalize_new(&Cartesian3::new(1.0, -1.0, 1.0));
    let angle = CesiumMath::PI_OVER_FOUR;
    let q = Quaternion::from_axis_angle_new(&axis, angle);
    let result = Quaternion::log_new(&q);
    let expected = Cartesian3::multiply_by_scalar_new(&axis, angle * 0.5);
    assert!(result.equals_epsilon_method(&expected, None, Some(CesiumMath::EPSILON15)));
}

#[test]
fn exp_works() {
    let axis = Cartesian3::normalize_new(&Cartesian3::new(1.0, -1.0, 1.0));
    let angle = CesiumMath::PI_OVER_FOUR;
    let cartesian = Cartesian3::multiply_by_scalar_new(&axis, angle * 0.5);
    let result = Quaternion::exp_new(&cartesian);
    let expected = Quaternion::from_axis_angle_new(&axis, angle);
    assert!(Quaternion::equals_epsilon(&result, &expected, CesiumMath::EPSILON15));
}

// --- squad / computeInnerQuadrangle ---

#[test]
fn squad_and_compute_inner_quadrangle_work() {
    let q0 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, 0.0);
    let q1 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, CesiumMath::PI_OVER_FOUR);
    let q2 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_Z, CesiumMath::PI_OVER_FOUR);
    let q3 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, -CesiumMath::PI_OVER_FOUR);

    let s1 = Quaternion::compute_inner_quadrangle_new(&q0, &q1, &q2);
    let s2 = Quaternion::compute_inner_quadrangle_new(&q1, &q2, &q3);

    let squad_result = Quaternion::squad_new(&q1, &q2, &s1, &s2, 0.0);
    assert!(Quaternion::equals_epsilon(&squad_result, &q1, CesiumMath::EPSILON15));
}

// --- fastSlerp ---

#[test]
fn fast_slerp_works() {
    let start = Quaternion::normalize_new(&Quaternion::new(0.0, 0.0, 0.0, 1.0));
    let end = Quaternion::new(0.0, 0.0, CesiumMath::PI_OVER_FOUR.sin(), CesiumMath::PI_OVER_FOUR.cos());
    let expected = Quaternion::new(0.0, 0.0, (std::f64::consts::PI / 8.0).sin(), (std::f64::consts::PI / 8.0).cos());
    let result = Quaternion::fast_slerp_new(&start, &end, 0.5);
    assert!(Quaternion::equals_epsilon(&result, &expected, CesiumMath::EPSILON6));
}

#[test]
fn fast_slerp_obtuse_angles() {
    let start = Quaternion::normalize_new(&Quaternion::new(0.0, 0.0, 0.0, -1.0));
    let end = Quaternion::new(0.0, 0.0, CesiumMath::PI_OVER_FOUR.sin(), CesiumMath::PI_OVER_FOUR.cos());
    let expected = Quaternion::new(0.0, 0.0, -(std::f64::consts::PI / 8.0).sin(), -(std::f64::consts::PI / 8.0).cos());
    let result = Quaternion::fast_slerp_new(&start, &end, 0.5);
    assert!(Quaternion::equals_epsilon(&result, &expected, CesiumMath::EPSILON6));
}

// --- fastSquad ---

#[test]
fn fast_squad_works() {
    let q0 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, 0.0);
    let q1 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, CesiumMath::PI_OVER_FOUR);
    let q2 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_Z, CesiumMath::PI_OVER_FOUR);
    let q3 = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, -CesiumMath::PI_OVER_FOUR);

    let s1 = Quaternion::compute_inner_quadrangle_new(&q0, &q1, &q2);
    let s2 = Quaternion::compute_inner_quadrangle_new(&q1, &q2, &q3);

    let result = Quaternion::fast_squad_new(&q1, &q2, &s1, &s2, 0.0);
    assert!(Quaternion::equals_epsilon(&result, &q1, CesiumMath::EPSILON6));
}

// --- equals / equalsEpsilon ---

#[test]
fn equals_works() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    assert!(Quaternion::equals(&q, &Quaternion::new(1.0, 2.0, 3.0, 4.0)));
    assert!(!Quaternion::equals(&q, &Quaternion::new(2.0, 2.0, 3.0, 4.0)));
    assert!(!Quaternion::equals(&q, &Quaternion::new(1.0, 3.0, 3.0, 4.0)));
    assert!(!Quaternion::equals(&q, &Quaternion::new(1.0, 2.0, 4.0, 4.0)));
    assert!(!Quaternion::equals(&q, &Quaternion::new(1.0, 2.0, 3.0, 5.0)));
}

#[test]
fn equals_epsilon_works() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    assert!(Quaternion::equals_epsilon(&q, &Quaternion::new(1.0, 2.0, 3.0, 4.0), 0.0));
    assert!(Quaternion::equals_epsilon(&q, &Quaternion::new(2.0, 2.0, 3.0, 4.0), 1.0));
    assert!(!Quaternion::equals_epsilon(&q, &Quaternion::new(2.0, 2.0, 3.0, 4.0), 0.99999));
}

// --- toString ---

#[test]
fn to_string_works() {
    let q = Quaternion::new(1.123, 2.345, 6.789, 6.123);
    assert_eq!(format!("{}", q), "(1.123, 2.345, 6.789, 6.123)");
}

// --- pack / unpack ---

#[test]
fn pack_unpack_roundtrip() {
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let mut array = [0.0; 4];
    Quaternion::pack(&q, &mut array, 0);
    let q2 = Quaternion::unpack_new(&array, 0);
    assert_eq!(q, q2);
}
