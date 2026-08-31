//! Mirrors packages/engine/Specs/Core/Matrix3Spec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::heading_pitch_roll::HeadingPitchRoll;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::quaternion::Quaternion;
use cesium_test_utils::assert_approx_eq_f64;

// --- constructor ---

#[test]
fn default_constructor_creates_zero_matrix() {
    let m = Matrix3::default();
    for i in 0..9 {
        assert_eq!(m.elements[i], 0.0);
    }
}

#[test]
fn constructor_sets_properties() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    // Column-major: [col0r0, col0r1, col0r2, col1r0, col1r1, col1r2, col2r0, col2r1, col2r2]
    assert_eq!(m.elements[Matrix3::COLUMN0ROW0], 1.0);
    assert_eq!(m.elements[Matrix3::COLUMN1ROW0], 2.0);
    assert_eq!(m.elements[Matrix3::COLUMN2ROW0], 3.0);
    assert_eq!(m.elements[Matrix3::COLUMN0ROW1], 4.0);
    assert_eq!(m.elements[Matrix3::COLUMN1ROW1], 5.0);
    assert_eq!(m.elements[Matrix3::COLUMN2ROW1], 6.0);
    assert_eq!(m.elements[Matrix3::COLUMN0ROW2], 7.0);
    assert_eq!(m.elements[Matrix3::COLUMN1ROW2], 8.0);
    assert_eq!(m.elements[Matrix3::COLUMN2ROW2], 9.0);
}

// --- fromArray / unpack ---

#[test]
fn from_array_works() {
    // fromArray uses column-major: [1,4,7,2,5,8,3,6,9]
    let expected = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let m = Matrix3::from_array_new(&[1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0], 0);
    assert_eq!(m, expected);
}

#[test]
fn from_array_with_starting_index() {
    let expected = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let m = Matrix3::from_array_new(&[0.0, 0.0, 1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0], 2);
    assert_eq!(m, expected);
}

// --- fromRowMajorArray ---

#[test]
fn from_row_major_array_works() {
    let expected = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let m = Matrix3::from_row_major_array_new(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    assert_eq!(m, expected);
}

// --- fromColumnMajorArray ---

#[test]
fn from_column_major_array_works() {
    let expected = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let m = Matrix3::from_column_major_array_new(&[1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]);
    assert_eq!(m, expected);
}

// --- fromScale ---

#[test]
fn from_scale_works() {
    let expected = Matrix3::new(7.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0, 9.0);
    let m = Matrix3::from_scale_new(&Cartesian3::new(7.0, 8.0, 9.0));
    assert_eq!(m, expected);
}

// --- fromUniformScale ---

#[test]
fn from_uniform_scale_works() {
    let expected = Matrix3::new(2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0);
    let m = Matrix3::from_uniform_scale_new(2.0);
    assert_eq!(m, expected);
}

// --- fromCrossProduct ---

#[test]
fn from_cross_product_works() {
    // For vector (7, 8, 9):
    // [ 0, -9,  8]
    // [ 9,  0, -7]
    // [-8,  7,  0]
    // Column-major: [0, 9, -8, -9, 0, 7, 8, -7, 0]
    let expected = Matrix3::new(0.0, -9.0, 8.0, 9.0, 0.0, -7.0, -8.0, 7.0, 0.0);
    let m = Matrix3::from_cross_product_new(&Cartesian3::new(7.0, 8.0, 9.0));
    assert_eq!(m, expected);
}

// --- fromRotationX ---

#[test]
fn from_rotation_x_works() {
    let angle = std::f64::consts::PI / 2.0;
    let m = Matrix3::from_rotation_x_new(angle);
    assert_approx_eq_f64!(m.elements[0], 1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[4], 0.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[5], 1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[7], -1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[8], 0.0, CesiumMath::EPSILON15);
}

// --- fromRotationY ---

#[test]
fn from_rotation_y_works() {
    let angle = std::f64::consts::PI / 2.0;
    let m = Matrix3::from_rotation_y_new(angle);
    assert_approx_eq_f64!(m.elements[0], 0.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[2], -1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[4], 1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[6], 1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[8], 0.0, CesiumMath::EPSILON15);
}

// --- fromRotationZ ---

#[test]
fn from_rotation_z_works() {
    let angle = std::f64::consts::PI / 2.0;
    let m = Matrix3::from_rotation_z_new(angle);
    assert_approx_eq_f64!(m.elements[0], 0.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[1], 1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[3], -1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[4], 0.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[8], 1.0, CesiumMath::EPSILON15);
}

// --- toArray ---

#[test]
fn to_array_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let arr = Matrix3::to_array_new(&m);
    // elements stored as [1,4,7,2,5,8,3,6,9] in column-major
    assert_eq!(arr, [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]);
}

// --- getElementIndex ---

#[test]
fn get_element_index_works() {
    assert_eq!(Matrix3::get_element_index(0, 0), Matrix3::COLUMN0ROW0);
    assert_eq!(Matrix3::get_element_index(0, 1), Matrix3::COLUMN0ROW1);
    assert_eq!(Matrix3::get_element_index(0, 2), Matrix3::COLUMN0ROW2);
    assert_eq!(Matrix3::get_element_index(1, 0), Matrix3::COLUMN1ROW0);
    assert_eq!(Matrix3::get_element_index(1, 1), Matrix3::COLUMN1ROW1);
    assert_eq!(Matrix3::get_element_index(1, 2), Matrix3::COLUMN1ROW2);
    assert_eq!(Matrix3::get_element_index(2, 0), Matrix3::COLUMN2ROW0);
    assert_eq!(Matrix3::get_element_index(2, 1), Matrix3::COLUMN2ROW1);
    assert_eq!(Matrix3::get_element_index(2, 2), Matrix3::COLUMN2ROW2);
}

// --- getColumn / setColumn ---

#[test]
fn get_column_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    // elements = [1,4,7,2,5,8,3,6,9]
    // col0 = elements[0..2] = (1,4,7)
    let col0 = Matrix3::get_column_new(&m, 0);
    assert_eq!(col0, Cartesian3::new(1.0, 4.0, 7.0));
    let col1 = Matrix3::get_column_new(&m, 1);
    assert_eq!(col1, Cartesian3::new(2.0, 5.0, 8.0));
    let col2 = Matrix3::get_column_new(&m, 2);
    assert_eq!(col2, Cartesian3::new(3.0, 6.0, 9.0));
}

#[test]
fn set_column_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let mut result = Matrix3::default();
    Matrix3::set_column(&m, 0, &Cartesian3::new(10.0, 11.0, 12.0), &mut result);
    // col0 replaced: elements = [10,11,12,2,5,8,3,6,9]
    // → new(10,2,3,11,5,6,12,8,9)
    assert_eq!(result, Matrix3::new(10.0, 2.0, 3.0, 11.0, 5.0, 6.0, 12.0, 8.0, 9.0));
}

// --- getRow / setRow ---

#[test]
fn get_row_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    // elements = [1,4,7,2,5,8,3,6,9]
    // row0: elements[0], elements[3], elements[6] = 1, 2, 3
    let row0 = Matrix3::get_row_new(&m, 0);
    assert_eq!(row0, Cartesian3::new(1.0, 2.0, 3.0));
    let row1 = Matrix3::get_row_new(&m, 1);
    assert_eq!(row1, Cartesian3::new(4.0, 5.0, 6.0));
    let row2 = Matrix3::get_row_new(&m, 2);
    assert_eq!(row2, Cartesian3::new(7.0, 8.0, 9.0));
}

#[test]
fn set_row_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let mut result = Matrix3::default();
    Matrix3::set_row(&m, 0, &Cartesian3::new(10.0, 11.0, 12.0), &mut result);
    // row0 replaced: elements[0]=10, elements[3]=11, elements[6]=12
    // elements = [10,4,7,11,5,8,12,6,9]
    // → new(10,11,12,4,5,6,7,8,9)
    assert_eq!(result, Matrix3::new(10.0, 11.0, 12.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0));
}

// --- getScale ---

#[test]
fn get_scale_works() {
    let m = Matrix3::from_scale_new(&Cartesian3::new(7.0, 8.0, 9.0));
    let scale = Matrix3::get_scale_new(&m);
    assert_approx_eq_f64!(scale.x, 7.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(scale.y, 8.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(scale.z, 9.0, CesiumMath::EPSILON14);
}

#[test]
fn get_maximum_scale_works() {
    let m = Matrix3::from_scale_new(&Cartesian3::new(7.0, 8.0, 9.0));
    let max_scale = Matrix3::get_maximum_scale(&m);
    assert_approx_eq_f64!(max_scale, 9.0, CesiumMath::EPSILON14);
}

// --- multiply ---

#[test]
fn multiply_works() {
    let left = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let right = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    let result = Matrix3::multiply_new(&left, &right);
    assert_eq!(result, left);
}

#[test]
fn multiply_by_identity_returns_same() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let result = Matrix3::multiply_new(&m, &Matrix3::IDENTITY);
    assert_eq!(result, m);
}

// --- add / subtract ---

#[test]
fn add_works() {
    let left = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let right = Matrix3::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
    let result = Matrix3::add_new(&left, &right);
    assert_eq!(result, Matrix3::new(2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0));
}

#[test]
fn subtract_works() {
    let left = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let right = Matrix3::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
    let result = Matrix3::subtract_new(&left, &right);
    assert_eq!(result, Matrix3::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0));
}

// --- multiplyByVector ---

#[test]
fn multiply_by_vector_works() {
    let m = Matrix3::from_scale_new(&Cartesian3::new(2.0, 3.0, 4.0));
    let v = Cartesian3::new(1.0, 2.0, 3.0);
    let result = Matrix3::multiply_by_vector_new(&m, &v);
    assert_eq!(result, Cartesian3::new(2.0, 6.0, 12.0));
}

// --- multiplyByScalar ---

#[test]
fn multiply_by_scalar_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let result = Matrix3::multiply_by_scalar_new(&m, 2.0);
    assert_eq!(result, Matrix3::new(2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0));
}

// --- multiplyByScale ---

#[test]
fn multiply_by_scale_works() {
    let m = Matrix3::from_scale_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let scale = Cartesian3::new(2.0, 3.0, 4.0);
    let mut result = Matrix3::default();
    Matrix3::multiply_by_scale(&m, &scale, &mut result);
    assert_eq!(result, Matrix3::from_scale_new(&Cartesian3::new(2.0, 6.0, 12.0)));
}

// --- multiplyByUniformScale ---

#[test]
fn multiply_by_uniform_scale_works() {
    let m = Matrix3::from_scale_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let mut result = Matrix3::default();
    Matrix3::multiply_by_uniform_scale(&m, 2.0, &mut result);
    assert_eq!(result, Matrix3::from_scale_new(&Cartesian3::new(2.0, 4.0, 6.0)));
}

// --- negate ---

#[test]
fn negate_works() {
    let m = Matrix3::new(1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0, 9.0);
    let result = Matrix3::negate_new(&m);
    assert_eq!(result, Matrix3::new(-1.0, 2.0, -3.0, 4.0, -5.0, 6.0, -7.0, 8.0, -9.0));
}

// --- transpose ---

#[test]
fn transpose_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let result = Matrix3::transpose_new(&m);
    // Transpose swaps rows and columns
    // Original: col0=(1,2,3), col1=(4,5,6), col2=(7,8,9)
    // Transposed: col0=(1,4,7), col1=(2,5,8), col2=(3,6,9)
    assert_eq!(result, Matrix3::new(1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0));
}

// --- abs ---

#[test]
fn abs_works() {
    let m = Matrix3::new(-1.0, 2.0, -3.0, 4.0, -5.0, 6.0, -7.0, 8.0, -9.0);
    let result = Matrix3::abs_new(&m);
    assert_eq!(result, Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0));
}

// --- determinant ---

#[test]
fn determinant_works() {
    let m = Matrix3::from_scale_new(&Cartesian3::new(2.0, 3.0, 4.0));
    assert_approx_eq_f64!(Matrix3::determinant(&m), 24.0, CesiumMath::EPSILON14);
}

#[test]
fn determinant_of_identity_is_one() {
    assert_approx_eq_f64!(Matrix3::determinant(&Matrix3::IDENTITY), 1.0, CesiumMath::EPSILON14);
}

// --- inverse ---

#[test]
fn inverse_works() {
    let m = Matrix3::from_scale_new(&Cartesian3::new(2.0, 4.0, 8.0));
    let inv = Matrix3::inverse_new(&m).unwrap();
    let product = Matrix3::multiply_new(&m, &inv);
    assert!(Matrix3::equals_epsilon(&product, &Matrix3::IDENTITY, CesiumMath::EPSILON14));
}

#[test]
fn inverse_of_non_invertible_returns_none() {
    let m = Matrix3::ZERO;
    assert!(Matrix3::inverse_new(&m).is_none());
}

// --- equals / equalsEpsilon ---

#[test]
fn equals_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    assert!(Matrix3::equals(&m, &Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0)));
    assert!(!Matrix3::equals(&m, &Matrix3::new(0.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0)));
}

#[test]
fn equals_epsilon_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let close = Matrix3::new(1.0 + 1e-14, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    assert!(Matrix3::equals_epsilon(&m, &close, CesiumMath::EPSILON10));
    assert!(!Matrix3::equals_epsilon(&m, &close, 0.0));
}

// --- clone ---

#[test]
fn clone_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let result = Matrix3::clone_new(&m);
    assert_eq!(result, m);
}

// --- pack / unpack ---

#[test]
fn pack_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let mut array = [0.0; 9];
    Matrix3::pack(&m, &mut array, 0);
    // elements = [1,4,7,2,5,8,3,6,9]
    assert_eq!(array, [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]);
}

#[test]
fn unpack_works() {
    let array = [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0];
    let m = Matrix3::unpack_new(&array, 0);
    assert_eq!(m, Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0));
}

// --- getRotation ---

#[test]
fn get_rotation_works() {
    let angle = std::f64::consts::PI / 4.0;
    let m = Matrix3::from_rotation_z_new(angle);
    let rotation = Matrix3::get_rotation_new(&m);
    assert!(Matrix3::equals_epsilon(&m, &rotation, CesiumMath::EPSILON14));
}

// --- toString ---

#[test]
fn to_string_works() {
    let m = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
    let s = m.to_string();
    // elements = [1,4,7,2,5,8,3,6,9]
    // toString: (e0, e3, e6)\n(e1, e4, e7)\n(e2, e5, e8)
    assert_eq!(s, "(1, 2, 3)\n(4, 5, 6)\n(7, 8, 9)");
}

// --- IDENTITY / ZERO ---

#[test]
fn identity_is_correct() {
    assert_eq!(Matrix3::IDENTITY, Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0));
}

#[test]
fn zero_is_correct() {
    assert_eq!(Matrix3::ZERO, Matrix3::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
}

// --- computeEigenDecomposition ---

#[test]
fn compute_eigen_decomposition_works() {
    // Symmetric matrix
    let m = Matrix3::new(2.0, 1.0, 0.0, 1.0, 3.0, 1.0, 0.0, 1.0, 2.0);
    let result = Matrix3::compute_eigen_decomposition(&m, None);

    // Verify: diagonal should have eigenvalues, unitary should be orthogonal
    // unitary * diagonal * transpose(unitary) ≈ original
    let mut temp = Matrix3::default();
    Matrix3::multiply(&result.unitary, &result.diagonal, &mut temp);
    let mut ut = Matrix3::default();
    Matrix3::transpose(&result.unitary, &mut ut);
    let mut reconstructed = Matrix3::default();
    Matrix3::multiply(&temp, &ut, &mut reconstructed);

    assert!(Matrix3::equals_epsilon(&reconstructed, &m, CesiumMath::EPSILON10));
}

// --- fromHeadingPitchRoll ---
// Mirrors Matrix3Spec.js "fromHeadingPitchRoll works without a result
// parameter" / "... with a result parameter" / "... computed correctly".
// DEVIATION: the JS "throws without quaternion parameter" case passes
// `undefined`, which the non-optional Rust parameter cannot express.

#[test]
fn from_heading_pitch_roll_works_without_result_parameter() {
    let s_pi_over_4 = CesiumMath::PI_OVER_FOUR.sin();
    let c_pi_over_4 = CesiumMath::PI_OVER_FOUR.cos();
    let s_pi_over_2 = CesiumMath::PI_OVER_TWO.sin();
    let c_pi_over_2 = CesiumMath::PI_OVER_TWO.cos();

    let tmp = Cartesian3::multiply_by_scalar_new(
        &Cartesian3::new(0.0, 0.0, 1.0),
        s_pi_over_4,
    );
    let quaternion = Quaternion::new(tmp.x, tmp.y, tmp.z, c_pi_over_4);
    let heading_pitch_roll = HeadingPitchRoll::from_quaternion_new(&quaternion);
    let expected = Matrix3::new(
        c_pi_over_2,
        -s_pi_over_2,
        0.0,
        s_pi_over_2,
        c_pi_over_2,
        0.0,
        0.0,
        0.0,
        1.0,
    );

    let returned_result = Matrix3::from_heading_pitch_roll_new(&heading_pitch_roll);
    assert!(Matrix3::equals_epsilon(
        &returned_result,
        &expected,
        CesiumMath::EPSILON15
    ));
}

#[test]
fn from_heading_pitch_roll_works_with_result_parameter() {
    let s_pi_over_4 = CesiumMath::PI_OVER_FOUR.sin();
    let c_pi_over_4 = CesiumMath::PI_OVER_FOUR.cos();
    let s_pi_over_2 = CesiumMath::PI_OVER_TWO.sin();
    let c_pi_over_2 = CesiumMath::PI_OVER_TWO.cos();

    let tmp = Cartesian3::multiply_by_scalar_new(
        &Cartesian3::new(0.0, 0.0, 1.0),
        s_pi_over_4,
    );
    let quaternion = Quaternion::new(tmp.x, tmp.y, tmp.z, c_pi_over_4);
    let heading_pitch_roll = HeadingPitchRoll::from_quaternion_new(&quaternion);
    let expected = Matrix3::new(
        c_pi_over_2,
        -s_pi_over_2,
        0.0,
        s_pi_over_2,
        c_pi_over_2,
        0.0,
        0.0,
        0.0,
        1.0,
    );

    let mut result = Matrix3::default();
    Matrix3::from_heading_pitch_roll(&heading_pitch_roll, &mut result);
    assert!(Matrix3::equals_epsilon(
        &result,
        &expected,
        CesiumMath::EPSILON15
    ));
}

#[test]
fn from_heading_pitch_roll_computed_correctly() {
    // Expected generated via STK Components (mirrors the JS spec verbatim).
    let expected = Matrix3::new(
        0.754406506735489,
        0.418940943945763,
        0.505330889696038,
        0.133022221559489,
        0.656295369162553,
        -0.742685314912828,
        -0.642787609686539,
        0.627506871597133,
        0.439385041770705,
    );

    let heading_pitch_roll = HeadingPitchRoll::new(
        -CesiumMath::to_radians(10.0),
        -CesiumMath::to_radians(40.0),
        CesiumMath::to_radians(55.0),
    );
    let mut result = Matrix3::default();
    Matrix3::from_heading_pitch_roll(&heading_pitch_roll, &mut result);
    for i in 0..9 {
        assert_approx_eq_f64!(result.elements[i], expected.elements[i], CesiumMath::EPSILON15);
    }
}
