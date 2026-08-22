//! Mirrors packages/engine/Specs/Core/Matrix2Spec.js

use cesium_core::cartesian2::Cartesian2;
use cesium_core::math::CesiumMath;
use cesium_core::matrix2::Matrix2;
use cesium_test_utils::assert_approx_eq_f64;

// --- constructor ---

#[test]
fn default_constructor_creates_zero_matrix() {
    let m = Matrix2::default();
    assert_eq!(m.elements[Matrix2::COLUMN0ROW0], 0.0);
    assert_eq!(m.elements[Matrix2::COLUMN1ROW0], 0.0);
    assert_eq!(m.elements[Matrix2::COLUMN0ROW1], 0.0);
    assert_eq!(m.elements[Matrix2::COLUMN1ROW1], 0.0);
}

#[test]
fn constructor_sets_properties() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(m.elements[Matrix2::COLUMN0ROW0], 1.0);
    assert_eq!(m.elements[Matrix2::COLUMN1ROW0], 2.0);
    assert_eq!(m.elements[Matrix2::COLUMN0ROW1], 3.0);
    assert_eq!(m.elements[Matrix2::COLUMN1ROW1], 4.0);
}

// --- fromArray / unpack ---

#[test]
fn from_array_works() {
    let expected = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let m = Matrix2::from_array_new(&[1.0, 3.0, 2.0, 4.0], 0);
    assert_eq!(m, expected);
}

#[test]
fn from_array_with_starting_index() {
    let expected = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let m = Matrix2::from_array_new(&[0.0, 0.0, 0.0, 1.0, 3.0, 2.0, 4.0], 3);
    assert_eq!(m, expected);
}

// --- fromRowMajorArray ---

#[test]
fn from_row_major_array_works() {
    let expected = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let m = Matrix2::from_row_major_array_new(&[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(m, expected);
}

// --- fromColumnMajorArray ---

#[test]
fn from_column_major_array_works() {
    let expected = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let m = Matrix2::from_column_major_array_new(&[1.0, 3.0, 2.0, 4.0]);
    assert_eq!(m, expected);
}

// --- fromScale ---

#[test]
fn from_scale_works() {
    let expected = Matrix2::new(7.0, 0.0, 0.0, 8.0);
    let m = Matrix2::from_scale_new(&Cartesian2::new(7.0, 8.0));
    assert_eq!(m, expected);
}

// --- fromUniformScale ---

#[test]
fn from_uniform_scale_works() {
    let expected = Matrix2::new(2.0, 0.0, 0.0, 2.0);
    let m = Matrix2::from_uniform_scale_new(2.0);
    assert_eq!(m, expected);
}

// --- fromRotation ---

#[test]
fn from_rotation_works() {
    let angle = std::f64::consts::PI / 2.0;
    let m = Matrix2::from_rotation_new(angle);
    assert_approx_eq_f64!(m.elements[0], 0.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[1], 1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[2], -1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[3], 0.0, CesiumMath::EPSILON15);
}

// --- toArray ---

#[test]
fn to_array_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let arr = Matrix2::to_array_new(&m);
    // new(1,2,3,4) → elements [1,3,2,4] (column-major)
    assert_eq!(arr, [1.0, 3.0, 2.0, 4.0]);
}

// --- getElementIndex ---

#[test]
fn get_element_index_works() {
    assert_eq!(Matrix2::get_element_index(0, 0), Matrix2::COLUMN0ROW0);
    assert_eq!(Matrix2::get_element_index(0, 1), Matrix2::COLUMN0ROW1);
    assert_eq!(Matrix2::get_element_index(1, 0), Matrix2::COLUMN1ROW0);
    assert_eq!(Matrix2::get_element_index(1, 1), Matrix2::COLUMN1ROW1);
}

// --- getColumn / setColumn ---

#[test]
fn get_column_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    // new(1,2,3,4) → elements [1,3,2,4]; col0 = elements[0..1] = (1,3)
    let col0 = Matrix2::get_column_new(&m, 0);
    assert_eq!(col0, Cartesian2::new(1.0, 3.0));
    let col1 = Matrix2::get_column_new(&m, 1);
    assert_eq!(col1, Cartesian2::new(2.0, 4.0));
}

#[test]
fn set_column_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let mut result = Matrix2::default();
    Matrix2::set_column(&m, 0, &Cartesian2::new(5.0, 6.0), &mut result);
    // col0 replaced → elements [5,6,2,4] → new(5,2,6,4)
    assert_eq!(result, Matrix2::new(5.0, 2.0, 6.0, 4.0));
}

// --- getRow / setRow ---

#[test]
fn get_row_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    // new(1,2,3,4) → visual matrix [[1,2],[3,4]]; row0 = (1,2), row1 = (3,4)
    let row0 = Matrix2::get_row_new(&m, 0);
    assert_eq!(row0, Cartesian2::new(1.0, 2.0));
    let row1 = Matrix2::get_row_new(&m, 1);
    assert_eq!(row1, Cartesian2::new(3.0, 4.0));
}

#[test]
fn set_row_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let mut result = Matrix2::default();
    Matrix2::set_row(&m, 0, &Cartesian2::new(5.0, 6.0), &mut result);
    // row0 replaced → elements [5,3,6,4] → new(5,6,3,4)
    assert_eq!(result, Matrix2::new(5.0, 6.0, 3.0, 4.0));
}

// --- getScale / setScale ---

#[test]
fn get_scale_works() {
    let m = Matrix2::from_scale_new(&Cartesian2::new(7.0, 8.0));
    let scale = Matrix2::get_scale_new(&m);
    assert_approx_eq_f64!(scale.x, 7.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(scale.y, 8.0, CesiumMath::EPSILON14);
}

#[test]
fn get_maximum_scale_works() {
    let m = Matrix2::from_scale_new(&Cartesian2::new(7.0, 8.0));
    let max_scale = Matrix2::get_maximum_scale(&m);
    assert_approx_eq_f64!(max_scale, 8.0, CesiumMath::EPSILON14);
}

// --- multiply ---

#[test]
fn multiply_works() {
    let left = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let right = Matrix2::new(5.0, 6.0, 7.0, 8.0);
    let result = Matrix2::multiply_new(&left, &right);
    // left.elems=[1,3,2,4], right.elems=[5,7,6,8]
    // col0row0 = 1*5 + 2*7 = 19
    // col0row1 = 3*5 + 4*7 = 43
    // col1row0 = 1*6 + 2*8 = 22
    // col1row1 = 3*6 + 4*8 = 50
    assert_eq!(result, Matrix2::new(19.0, 22.0, 43.0, 50.0));
}

#[test]
fn multiply_by_identity_returns_same() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let result = Matrix2::multiply_new(&m, &Matrix2::IDENTITY);
    assert_eq!(result, m);
}

// --- add / subtract ---

#[test]
fn add_works() {
    let left = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let right = Matrix2::new(5.0, 6.0, 7.0, 8.0);
    let result = Matrix2::add_new(&left, &right);
    assert_eq!(result, Matrix2::new(6.0, 8.0, 10.0, 12.0));
}

#[test]
fn subtract_works() {
    let left = Matrix2::new(5.0, 6.0, 7.0, 8.0);
    let right = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let result = Matrix2::subtract_new(&left, &right);
    assert_eq!(result, Matrix2::new(4.0, 4.0, 4.0, 4.0));
}

// --- multiplyByVector ---

#[test]
fn multiply_by_vector_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let v = Cartesian2::new(5.0, 6.0);
    let result = Matrix2::multiply_by_vector_new(&m, &v);
    // m.elems=[1,3,2,4]; x = 1*5 + 2*6 = 17; y = 3*5 + 4*6 = 39
    assert_eq!(result, Cartesian2::new(17.0, 39.0));
}

// --- multiplyByScalar ---

#[test]
fn multiply_by_scalar_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let result = Matrix2::multiply_by_scalar_new(&m, 2.0);
    assert_eq!(result, Matrix2::new(2.0, 4.0, 6.0, 8.0));
}

// --- multiplyByScale ---

#[test]
fn multiply_by_scale_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let scale = Cartesian2::new(2.0, 3.0);
    let mut result = Matrix2::default();
    Matrix2::multiply_by_scale(&m, &scale, &mut result);
    // m.elems=[1,3,2,4]; col0 scaled by 2 → [2,6], col1 scaled by 3 → [6,12]
    assert_eq!(result, Matrix2::new(2.0, 6.0, 6.0, 12.0));
}

// --- multiplyByUniformScale ---

#[test]
fn multiply_by_uniform_scale_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let mut result = Matrix2::default();
    Matrix2::multiply_by_uniform_scale(&m, 2.0, &mut result);
    assert_eq!(result, Matrix2::new(2.0, 4.0, 6.0, 8.0));
}

// --- negate ---

#[test]
fn negate_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let result = Matrix2::negate_new(&m);
    assert_eq!(result, Matrix2::new(-1.0, -2.0, -3.0, -4.0));
}

// --- transpose ---

#[test]
fn transpose_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let result = Matrix2::transpose_new(&m);
    assert_eq!(result, Matrix2::new(1.0, 3.0, 2.0, 4.0));
}

// --- abs ---

#[test]
fn abs_works() {
    let m = Matrix2::new(-1.0, -2.0, 3.0, 4.0);
    let result = Matrix2::abs_new(&m);
    assert_eq!(result, Matrix2::new(1.0, 2.0, 3.0, 4.0));
}

// --- equals / equalsEpsilon ---

#[test]
fn equals_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    assert!(Matrix2::equals(&m, &Matrix2::new(1.0, 2.0, 3.0, 4.0)));
    assert!(!Matrix2::equals(&m, &Matrix2::new(5.0, 2.0, 3.0, 4.0)));
}

#[test]
fn equals_epsilon_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let close = Matrix2::new(1.0 + 1e-14, 2.0, 3.0, 4.0);
    assert!(Matrix2::equals_epsilon(&m, &close, CesiumMath::EPSILON10));
    assert!(!Matrix2::equals_epsilon(&m, &close, 0.0));
}

// --- clone ---

#[test]
fn clone_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let result = Matrix2::clone_new(&m);
    assert_eq!(result, m);
}

// --- pack / unpack ---

#[test]
fn pack_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let mut array = [0.0; 4];
    Matrix2::pack(&m, &mut array, 0);
    // new(1,2,3,4) → elements [1,3,2,4]
    assert_eq!(array, [1.0, 3.0, 2.0, 4.0]);
}

#[test]
fn unpack_works() {
    let array = [1.0, 3.0, 2.0, 4.0];
    let m = Matrix2::unpack_new(&array, 0);
    // unpack [1,3,2,4] → new(1,2,3,4) since elements = [col0row0, col0row1, col1row0, col1row1]
    assert_eq!(m, Matrix2::new(1.0, 2.0, 3.0, 4.0));
}

// --- getRotation / setRotation ---

#[test]
fn get_rotation_works() {
    let angle = std::f64::consts::PI / 4.0;
    let m = Matrix2::from_rotation_new(angle);
    let rotation = Matrix2::get_rotation_new(&m);
    assert!(Matrix2::equals_epsilon(&m, &rotation, CesiumMath::EPSILON14));
}

// --- toString ---

#[test]
fn to_string_works() {
    let m = Matrix2::new(1.0, 2.0, 3.0, 4.0);
    let s = m.to_string();
    assert_eq!(s, "(1, 2)\n(3, 4)");
}

// --- IDENTITY / ZERO ---

#[test]
fn identity_is_correct() {
    assert_eq!(Matrix2::IDENTITY, Matrix2::new(1.0, 0.0, 0.0, 1.0));
}

#[test]
fn zero_is_correct() {
    assert_eq!(Matrix2::ZERO, Matrix2::new(0.0, 0.0, 0.0, 0.0));
}
