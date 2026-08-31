//! Mirrors packages/engine/Specs/Core/Matrix4Spec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::{CameraView, Matrix4, Viewport};
use cesium_core::quaternion::Quaternion;
use cesium_core::translation_rotation_scale::TranslationRotationScale;
use cesium_test_utils::assert_approx_eq_f64;
use cesium_test_utils::expect_to_throw_dev_error;

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

// =====================================================================
// CZ-06: projection matrix family (Matrix4Spec.js)
// =====================================================================

fn assert_matrix_eq_epsilon(actual: &Matrix4, expected: &Matrix4, epsilon: f64) {
    for i in 0..16 {
        assert_approx_eq_f64!(actual.elements[i], expected.elements[i], epsilon);
    }
}

#[test]
fn from_translation_rotation_scale_works_without_a_result_parameter() {
    let expected = Matrix4::new(
        7.0, 0.0, 0.0, 1.0,
        0.0, 0.0, 9.0, 2.0,
        0.0, -8.0, 0.0, 3.0,
        0.0, 0.0, 0.0, 1.0,
    );

    let trs = TranslationRotationScale::new(
        Cartesian3::new(1.0, 2.0, 3.0),
        Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, CesiumMath::to_radians(-90.0)),
        Cartesian3::new(7.0, 8.0, 9.0),
    );

    let returned_result = Matrix4::from_translation_rotation_scale_new(&trs);
    assert_matrix_eq_epsilon(&returned_result, &expected, CesiumMath::EPSILON14);
}

#[test]
fn from_translation_rotation_scale_works_with_a_result_parameter() {
    let expected = Matrix4::new(
        7.0, 0.0, 0.0, 1.0,
        0.0, 0.0, 9.0, 2.0,
        0.0, -8.0, 0.0, 3.0,
        0.0, 0.0, 0.0, 1.0,
    );

    let trs = TranslationRotationScale::new(
        Cartesian3::new(1.0, 2.0, 3.0),
        Quaternion::from_axis_angle_new(&Cartesian3::UNIT_X, CesiumMath::to_radians(-90.0)),
        Cartesian3::new(7.0, 8.0, 9.0),
    );

    let mut result = Matrix4::default();
    Matrix4::from_translation_rotation_scale(&trs, &mut result);
    assert_matrix_eq_epsilon(&result, &expected, CesiumMath::EPSILON14);
}

#[test]
fn compute_perspective_field_of_view_works() {
    let expected = Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, -1.222222222222222, -2.222222222222222,
        0.0, 0.0, -1.0, 0.0,
    );
    let mut result = Matrix4::default();
    Matrix4::compute_perspective_field_of_view(CesiumMath::PI_OVER_TWO, 1.0, 1.0, 10.0, &mut result);
    assert_matrix_eq_epsilon(&result, &expected, CesiumMath::EPSILON15);
}

#[test]
fn from_camera_works_without_a_result_parameter() {
    let expected = Matrix4::IDENTITY;
    let returned_result = Matrix4::from_camera_new(&CameraView {
        position: Cartesian3::ZERO,
        direction: Cartesian3::negate_new(&Cartesian3::UNIT_Z),
        up: Cartesian3::UNIT_Y,
    });
    assert_eq!(expected, returned_result);
}

#[test]
fn from_camera_works_with_a_result_parameter() {
    let expected = Matrix4::IDENTITY;
    let mut result = Matrix4::default();
    Matrix4::from_camera(
        &CameraView {
            position: Cartesian3::ZERO,
            direction: Cartesian3::negate_new(&Cartesian3::UNIT_Z),
            up: Cartesian3::UNIT_Y,
        },
        &mut result,
    );
    assert_eq!(expected, result);
}

#[test]
fn compute_orthographic_off_center_works() {
    let expected = Matrix4::new(
        2.0, 0.0, 0.0, -1.0,
        0.0, 2.0, 0.0, -5.0,
        0.0, 0.0, -2.0, -1.0,
        0.0, 0.0, 0.0, 1.0,
    );
    let mut result = Matrix4::default();
    Matrix4::compute_orthographic_off_center(0.0, 1.0, 2.0, 3.0, 0.0, 1.0, &mut result);
    assert_eq!(expected, result);
}

#[test]
fn compute_viewport_transformation_works_without_a_result_parameter() {
    let expected = Matrix4::new(
        2.0, 0.0, 0.0, 2.0,
        0.0, 3.0, 0.0, 3.0,
        0.0, 0.0, 1.0, 1.0,
        0.0, 0.0, 0.0, 1.0,
    );
    let returned_result = Matrix4::compute_viewport_transformation_new(
        Some(&Viewport {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 6.0,
        }),
        Some(0.0),
        Some(2.0),
    );
    assert_eq!(expected, returned_result);
}

#[test]
fn compute_viewport_transformation_works_with_a_result_parameter() {
    let expected = Matrix4::new(
        2.0, 0.0, 0.0, 2.0,
        0.0, 3.0, 0.0, 3.0,
        0.0, 0.0, 1.0, 1.0,
        0.0, 0.0, 0.0, 1.0,
    );
    let mut result = Matrix4::default();
    Matrix4::compute_viewport_transformation(
        Some(&Viewport {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 6.0,
        }),
        Some(0.0),
        Some(2.0),
        &mut result,
    );
    assert_eq!(expected, result);
}

#[test]
fn compute_perspective_off_center_works() {
    let expected = Matrix4::new(
        2.0, 0.0, 3.0, 0.0,
        0.0, 2.0, 5.0, 0.0,
        0.0, 0.0, -3.0, -4.0,
        0.0, 0.0, -1.0, 0.0,
    );
    let mut result = Matrix4::default();
    Matrix4::compute_perspective_off_center(1.0, 2.0, 2.0, 3.0, 1.0, 2.0, &mut result);
    assert_eq!(expected, result);
}

#[test]
fn compute_infinite_perspective_off_center_works() {
    let expected = Matrix4::new(
        2.0, 0.0, 3.0, 0.0,
        0.0, 2.0, 5.0, 0.0,
        0.0, 0.0, -1.0, -2.0,
        0.0, 0.0, -1.0, 0.0,
    );
    let mut result = Matrix4::default();
    Matrix4::compute_infinite_perspective_off_center(1.0, 2.0, 2.0, 3.0, 1.0, &mut result);
    assert_eq!(expected, result);
}

#[test]
fn compute_perspective_field_of_view_throws_with_out_of_range_y_field_of_view() {
    expect_to_throw_dev_error(|| {
        let mut result = Matrix4::default();
        Matrix4::compute_perspective_field_of_view(0.0, 1.0, 2.0, 3.0, &mut result);
    });
}

// DEVIATION: the JS "out of range aspect" spec case only throws because the
// `result` parameter is omitted (Check.typeOf.object); the JS implementation
// has no aspectRatio range check, and Rust's signature makes the result
// parameter mandatory, so that case cannot be mirrored.

#[test]
fn compute_perspective_field_of_view_throws_with_out_of_range_near() {
    expect_to_throw_dev_error(|| {
        let mut result = Matrix4::default();
        Matrix4::compute_perspective_field_of_view(1.0, 1.0, 0.0, 3.0, &mut result);
    });
}

#[test]
fn compute_perspective_field_of_view_throws_with_out_of_range_far() {
    expect_to_throw_dev_error(|| {
        let mut result = Matrix4::default();
        Matrix4::compute_perspective_field_of_view(1.0, 1.0, 2.0, 0.0, &mut result);
    });
}

// DEVIATION: the JS "fromCamera throws without camera/position/direction/up"
// and the "throws without a result parameter" spec cases verify undefined
// checks that Rust's type system enforces at compile time; not mirrored.

#[test]
fn pack_array_round_trips_with_unpack_array() {
    let matrices = vec![
        Matrix4::new(
            1.0, 2.0, 3.0, 4.0,
            5.0, 6.0, 7.0, 8.0,
            9.0, 10.0, 11.0, 12.0,
            13.0, 14.0, 15.0, 16.0,
        ),
        Matrix4::new(
            17.0, 18.0, 19.0, 20.0,
            21.0, 22.0, 23.0, 24.0,
            25.0, 26.0, 27.0, 28.0,
            29.0, 30.0, 31.0, 32.0,
        ),
    ];
    let packed = Matrix4::pack_array_new(&matrices);
    assert_eq!(packed.len(), matrices.len() * 16);
    let unpacked = Matrix4::unpack_array_new(&packed);
    assert_eq!(unpacked.len(), matrices.len());
    assert_eq!(unpacked[0], matrices[0]);
    assert_eq!(unpacked[1], matrices[1]);
}

#[test]
fn unpack_array_throws_when_length_is_not_a_multiple_of_16() {
    expect_to_throw_dev_error(|| {
        let array = vec![0.0f64; 17];
        let _ = Matrix4::unpack_array_new(&array);
    });
}

#[test]
fn pack_array_into_throws_when_result_length_mismatches() {
    expect_to_throw_dev_error(|| {
        let matrices = vec![Matrix4::IDENTITY];
        let mut result = vec![0.0f64; 15];
        Matrix4::pack_array_into(&matrices, &mut result);
    });
}

#[test]
fn length_returns_packed_length() {
    let m = Matrix4::IDENTITY;
    assert_eq!(m.len(), Matrix4::PACKED_LENGTH);
}

#[test]
fn equals_array_compares_from_offset() {
    let m = Matrix4::new(
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    );
    let mut array = vec![0.0f64; 17];
    Matrix4::pack(&m, &mut array, 1);
    assert!(Matrix4::equals_array(&m, &array, 1));
    array[1] = 42.0;
    assert!(!Matrix4::equals_array(&m, &array, 1));
}
