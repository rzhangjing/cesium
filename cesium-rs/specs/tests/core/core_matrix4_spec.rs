//! Mirrors packages/engine/Specs/Core/Matrix4Spec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_test_utils::assert_approx_eq_f64;

// --- constructor ---

#[test]
fn default_constructor_creates_zero_matrix() {
    let m = Matrix4::default();
    for i in 0..16 {
        assert_eq!(m.elements[i], 0.0);
    }
}

#[test]
fn constructor_sets_properties() {
    let m = Matrix4::new(
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    );
    assert_eq!(m.elements[Matrix4::COLUMN0ROW0], 1.0);
    assert_eq!(m.elements[Matrix4::COLUMN1ROW0], 2.0);
    assert_eq!(m.elements[Matrix4::COLUMN2ROW0], 3.0);
    assert_eq!(m.elements[Matrix4::COLUMN3ROW0], 4.0);
    assert_eq!(m.elements[Matrix4::COLUMN0ROW1], 5.0);
    assert_eq!(m.elements[Matrix4::COLUMN1ROW1], 6.0);
    assert_eq!(m.elements[Matrix4::COLUMN2ROW1], 7.0);
    assert_eq!(m.elements[Matrix4::COLUMN3ROW1], 8.0);
    assert_eq!(m.elements[Matrix4::COLUMN0ROW2], 9.0);
    assert_eq!(m.elements[Matrix4::COLUMN1ROW2], 10.0);
    assert_eq!(m.elements[Matrix4::COLUMN2ROW2], 11.0);
    assert_eq!(m.elements[Matrix4::COLUMN3ROW2], 12.0);
    assert_eq!(m.elements[Matrix4::COLUMN0ROW3], 13.0);
    assert_eq!(m.elements[Matrix4::COLUMN1ROW3], 14.0);
    assert_eq!(m.elements[Matrix4::COLUMN2ROW3], 15.0);
    assert_eq!(m.elements[Matrix4::COLUMN3ROW3], 16.0);
}

// --- fromTranslation ---

#[test]
fn from_translation_works() {
    let m = Matrix4::from_translation_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let expected = Matrix4::new(
        1.0, 0.0, 0.0, 1.0,
        0.0, 1.0, 0.0, 2.0,
        0.0, 0.0, 1.0, 3.0,
        0.0, 0.0, 0.0, 1.0,
    );
    assert_eq!(m, expected);
}

// --- fromScale ---

#[test]
fn from_scale_works() {
    let m = Matrix4::from_scale_new(&Cartesian3::new(7.0, 8.0, 9.0));
    assert_eq!(m.elements[0], 7.0);
    assert_eq!(m.elements[5], 8.0);
    assert_eq!(m.elements[10], 9.0);
    assert_eq!(m.elements[15], 1.0);
}

// --- fromUniformScale ---

#[test]
fn from_uniform_scale_works() {
    let m = Matrix4::from_uniform_scale_new(2.0);
    assert_eq!(m.elements[0], 2.0);
    assert_eq!(m.elements[5], 2.0);
    assert_eq!(m.elements[10], 2.0);
    assert_eq!(m.elements[15], 1.0);
}

// --- fromRotation ---

#[test]
fn from_rotation_works() {
    let angle = std::f64::consts::PI / 2.0;
    let rot = Matrix3::from_rotation_z_new(angle);
    let m = Matrix4::from_rotation_new(&rot);
    assert_approx_eq_f64!(m.elements[0], 0.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[1], 1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[4], -1.0, CesiumMath::EPSILON15);
    assert_approx_eq_f64!(m.elements[5], 0.0, CesiumMath::EPSILON15);
    assert_eq!(m.elements[15], 1.0);
}

// --- fromRotationTranslation ---

#[test]
fn from_rotation_translation_works() {
    let rot = Matrix3::IDENTITY;
    let trans = Cartesian3::new(1.0, 2.0, 3.0);
    let m = Matrix4::from_rotation_translation_new(&rot, &trans);
    assert_eq!(m.elements[12], 1.0);
    assert_eq!(m.elements[13], 2.0);
    assert_eq!(m.elements[14], 3.0);
    assert_eq!(m.elements[0], 1.0);
    assert_eq!(m.elements[5], 1.0);
    assert_eq!(m.elements[10], 1.0);
}

// --- computeView ---

#[test]
fn compute_view_works() {
    let pos = Cartesian3::new(0.0, 0.0, 0.0);
    let dir = Cartesian3::new(0.0, 0.0, -1.0);
    let up = Cartesian3::new(0.0, 1.0, 0.0);
    let right = Cartesian3::new(1.0, 0.0, 0.0);
    let m = Matrix4::compute_view_new(&pos, &dir, &up, &right);
    // Should be identity-like since position is origin
    assert_eq!(m.elements[12], 0.0);
    assert_eq!(m.elements[13], 0.0);
    assert_eq!(m.elements[14], 0.0);
}

// --- getColumn / setColumn ---

#[test]
fn get_column_works() {
    let m = Matrix4::new(
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    );
    let col0 = Matrix4::get_column_new(&m, 0);
    assert_eq!(col0, Cartesian4::new(1.0, 5.0, 9.0, 13.0));
    let col3 = Matrix4::get_column_new(&m, 3);
    assert_eq!(col3, Cartesian4::new(4.0, 8.0, 12.0, 16.0));
}

// --- getRow / setRow ---

#[test]
fn get_row_works() {
    let m = Matrix4::new(
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    );
    // row0: elements[0], elements[4], elements[8], elements[12] = 1, 2, 3, 4
    let row0 = Matrix4::get_row_new(&m, 0);
    assert_eq!(row0, Cartesian4::new(1.0, 2.0, 3.0, 4.0));
}

// --- getTranslation / setTranslation ---

#[test]
fn get_translation_works() {
    let m = Matrix4::from_translation_new(&Cartesian3::new(10.0, 20.0, 30.0));
    let t = Matrix4::get_translation_new(&m);
    assert_eq!(t, Cartesian3::new(10.0, 20.0, 30.0));
}

// --- getScale ---

#[test]
fn get_scale_works() {
    let m = Matrix4::from_scale_new(&Cartesian3::new(7.0, 8.0, 9.0));
    let scale = Matrix4::get_scale_new(&m);
    assert_approx_eq_f64!(scale.x, 7.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(scale.y, 8.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(scale.z, 9.0, CesiumMath::EPSILON14);
}

// --- getRotation ---

#[test]
fn get_rotation_works() {
    let angle = std::f64::consts::PI / 4.0;
    let rot = Matrix3::from_rotation_z_new(angle);
    let m = Matrix4::from_rotation_new(&rot);
    let extracted = Matrix4::get_rotation_new(&m);
    assert!(Matrix3::equals_epsilon(&rot, &extracted, CesiumMath::EPSILON14));
}

// --- getMatrix3 ---

#[test]
fn get_matrix3_works() {
    let rot = Matrix3::from_rotation_z_new(std::f64::consts::PI / 3.0);
    let m = Matrix4::from_rotation_new(&rot);
    let m3 = Matrix4::get_matrix3_new(&m);
    assert!(Matrix3::equals_epsilon(&rot, &m3, CesiumMath::EPSILON14));
}

// --- multiply ---

#[test]
fn multiply_by_identity_returns_same() {
    let m = Matrix4::new(
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    );
    let result = Matrix4::multiply_new(&m, &Matrix4::IDENTITY);
    assert_eq!(result, m);
}

// --- multiplyByVector ---

#[test]
fn multiply_by_vector_works() {
    let m = Matrix4::from_scale_new(&Cartesian3::new(2.0, 3.0, 4.0));
    let v = Cartesian4::new(1.0, 2.0, 3.0, 1.0);
    let result = Matrix4::multiply_by_vector_new(&m, &v);
    assert_eq!(result, Cartesian4::new(2.0, 6.0, 12.0, 1.0));
}

// --- multiplyByPointAsVector ---

#[test]
fn multiply_by_point_as_vector_works() {
    // JS `multiplyByPointAsVector` applies only the upper-left 3x3 (no
    // translation); a pure translation leaves the vector unchanged.
    let m = Matrix4::from_translation_new(&Cartesian3::new(10.0, 20.0, 30.0));
    let p = Cartesian3::new(1.0, 2.0, 3.0);
    let result = Matrix4::multiply_by_point_as_vector_new(&m, &p);
    assert_eq!(result, Cartesian3::new(1.0, 2.0, 3.0));
}

// --- add / subtract ---

#[test]
fn add_works() {
    let a = Matrix4::from_uniform_scale_new(1.0);
    let b = Matrix4::from_uniform_scale_new(2.0);
    let result = Matrix4::add_new(&a, &b);
    assert_eq!(result.elements[0], 3.0);
    assert_eq!(result.elements[5], 3.0);
}

#[test]
fn subtract_works() {
    let a = Matrix4::from_uniform_scale_new(3.0);
    let b = Matrix4::from_uniform_scale_new(1.0);
    let result = Matrix4::subtract_new(&a, &b);
    assert_eq!(result.elements[0], 2.0);
    assert_eq!(result.elements[5], 2.0);
}

// --- transpose ---

#[test]
fn transpose_works() {
    let m = Matrix4::from_translation_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let t = Matrix4::transpose_new(&m);
    // Translation becomes row values after transpose
    assert_eq!(t.elements[3], 1.0);
    assert_eq!(t.elements[7], 2.0);
    assert_eq!(t.elements[11], 3.0);
}

// --- determinant ---

#[test]
fn determinant_of_identity_is_one() {
    assert_approx_eq_f64!(Matrix4::determinant(&Matrix4::IDENTITY), 1.0, CesiumMath::EPSILON14);
}

#[test]
fn determinant_of_scale() {
    let m = Matrix4::from_scale_new(&Cartesian3::new(2.0, 3.0, 4.0));
    assert_approx_eq_f64!(Matrix4::determinant(&m), 24.0, CesiumMath::EPSILON14);
}

// --- inverse ---

#[test]
fn inverse_of_identity_is_identity() {
    let inv = Matrix4::inverse_new(&Matrix4::IDENTITY).unwrap();
    assert!(Matrix4::equals_epsilon(&inv, &Matrix4::IDENTITY, CesiumMath::EPSILON14));
}

#[test]
fn inverse_times_matrix_is_identity() {
    let m = Matrix4::from_scale_new(&Cartesian3::new(2.0, 4.0, 8.0));
    let inv = Matrix4::inverse_new(&m).unwrap();
    let product = Matrix4::multiply_new(&m, &inv);
    assert!(Matrix4::equals_epsilon(&product, &Matrix4::IDENTITY, CesiumMath::EPSILON10));
}

#[test]
fn inverse_of_non_invertible_returns_none() {
    assert!(Matrix4::inverse_new(&Matrix4::ZERO).is_none());
}

// --- inverseTransformation ---

#[test]
fn inverse_transformation_works() {
    let m = Matrix4::from_translation_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let inv = Matrix4::inverse_transformation_new(&m);
    let product = Matrix4::multiply_new(&m, &inv);
    assert!(Matrix4::equals_epsilon(&product, &Matrix4::IDENTITY, CesiumMath::EPSILON14));
}

// --- equals / equalsEpsilon ---

#[test]
fn equals_works() {
    let m = Matrix4::IDENTITY;
    assert!(Matrix4::equals(&m, &Matrix4::IDENTITY));
    assert!(!Matrix4::equals(&m, &Matrix4::ZERO));
}

#[test]
fn equals_epsilon_works() {
    let m = Matrix4::IDENTITY;
    let close = Matrix4::new(
        1.0 + 1e-14, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );
    assert!(Matrix4::equals_epsilon(&m, &close, CesiumMath::EPSILON10));
    assert!(!Matrix4::equals_epsilon(&m, &close, 0.0));
}

// --- pack / unpack ---

#[test]
fn pack_unpack_roundtrip() {
    let m = Matrix4::from_translation_new(&Cartesian3::new(5.0, 6.0, 7.0));
    let mut array = [0.0; 16];
    Matrix4::pack(&m, &mut array, 0);
    let m2 = Matrix4::unpack_new(&array, 0);
    assert_eq!(m, m2);
}

// --- IDENTITY / ZERO ---

#[test]
fn identity_is_correct() {
    for i in 0..16 {
        if i % 5 == 0 {
            assert_eq!(Matrix4::IDENTITY.elements[i], 1.0);
        } else {
            assert_eq!(Matrix4::IDENTITY.elements[i], 0.0);
        }
    }
}
