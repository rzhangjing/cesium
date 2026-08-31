//! Mirrors packages/engine/Specs/Core/TransformsSpec.js

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::heading_pitch_roll::HeadingPitchRoll;
use cesium_core::julian_date::JulianDate;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::{CameraView, Matrix4, Viewport};
use cesium_core::quaternion::Quaternion;
use cesium_core::time_standard::TimeStandard;
use cesium_core::transforms;

fn get_col(m: &Matrix4, idx: usize) -> Cartesian4 {
    let mut c = Cartesian4::default();
    Matrix4::get_column(m, idx, &mut c);
    c
}

// ===== eastNorthUpToFixedFrame =====

#[test]
fn east_north_up_works_without_result() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let result = transforms::east_north_up_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Z); // north
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_X); // up
    assert_eq!(
        get_col(&result, 3),
        Cartesian4::new(origin.x, origin.y, origin.z, 1.0)
    );
}

#[test]
fn east_north_up_works_with_result() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let mut result = Matrix4::default();
    transforms::east_north_up_to_fixed_frame(&origin, Some(&Ellipsoid::UNIT_SPHERE), &mut result);

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Z); // north
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_X); // up
}

#[test]
fn east_north_up_works_at_north_pole() {
    let north_pole = Cartesian3::new(0.0, 0.0, 1.0);
    let result = transforms::east_north_up_to_fixed_frame_new(&north_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 1), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Z); // up
}

#[test]
fn east_north_up_works_at_south_pole() {
    let south_pole = Cartesian3::new(0.0, 0.0, -1.0);
    let result = transforms::east_north_up_to_fixed_frame_new(&south_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_X); // north
    assert_eq!(get_col(&result, 2), Cartesian4::new(0.0, 0.0, -1.0, 0.0)); // up
}

#[test]
fn east_north_up_works_at_origin() {
    let origin = Cartesian3::ZERO;
    let result = transforms::east_north_up_to_fixed_frame_new(&origin, Some(&Ellipsoid::WGS84));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Y); // east (degenerate)
    assert_eq!(get_col(&result, 1), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north (degenerate)
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Z); // up (degenerate)
}

// ===== northEastDownToFixedFrame =====

#[test]
fn north_east_down_works_without_result() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let result = transforms::north_east_down_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Z); // north
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 2), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // down
}

#[test]
fn north_east_down_works_at_north_pole() {
    let north_pole = Cartesian3::new(0.0, 0.0, 1.0);
    let result = transforms::north_east_down_to_fixed_frame_new(&north_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 2), Cartesian4::new(0.0, 0.0, -1.0, 0.0)); // down
}

#[test]
fn north_east_down_works_at_south_pole() {
    let south_pole = Cartesian3::new(0.0, 0.0, -1.0);
    let result = transforms::north_east_down_to_fixed_frame_new(&south_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_X); // north
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Z); // down
}

#[test]
fn north_east_down_works_at_origin() {
    let origin = Cartesian3::ZERO;
    let result = transforms::north_east_down_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Y); // east
    assert_eq!(get_col(&result, 2), Cartesian4::new(0.0, 0.0, -1.0, 0.0)); // down
}

// ===== northUpEastToFixedFrame =====

#[test]
fn north_up_east_works_without_result() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let result = transforms::north_up_east_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Z); // north
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_X); // up
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Y); // east
}

#[test]
fn north_up_east_works_at_north_pole() {
    let north_pole = Cartesian3::new(0.0, 0.0, 1.0);
    let result = transforms::north_up_east_to_fixed_frame_new(&north_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Z); // up
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Y); // east
}

#[test]
fn north_up_east_works_at_south_pole() {
    let south_pole = Cartesian3::new(0.0, 0.0, -1.0);
    let result = transforms::north_up_east_to_fixed_frame_new(&south_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_X); // north
    assert_eq!(get_col(&result, 1), Cartesian4::new(0.0, 0.0, -1.0, 0.0)); // up
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Y); // east
}

#[test]
fn north_up_east_works_at_origin() {
    let origin = Cartesian3::ZERO;
    let result = transforms::north_up_east_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north
    assert_eq!(get_col(&result, 1), Cartesian4::UNIT_Z); // up
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Y); // east
}

// ===== northWestUpToFixedFrame =====

#[test]
fn north_west_up_works_without_result() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let result = transforms::north_west_up_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_Z); // north
    assert_eq!(get_col(&result, 1), Cartesian4::new(0.0, -1.0, 0.0, 0.0)); // west
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_X); // up
}

#[test]
fn north_west_up_works_at_north_pole() {
    let north_pole = Cartesian3::new(0.0, 0.0, 1.0);
    let result = transforms::north_west_up_to_fixed_frame_new(&north_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north
    assert_eq!(get_col(&result, 1), Cartesian4::new(0.0, -1.0, 0.0, 0.0)); // west
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Z); // up
}

#[test]
fn north_west_up_works_at_south_pole() {
    let south_pole = Cartesian3::new(0.0, 0.0, -1.0);
    let result = transforms::north_west_up_to_fixed_frame_new(&south_pole, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::UNIT_X); // north
    assert_eq!(get_col(&result, 1), Cartesian4::new(0.0, -1.0, 0.0, 0.0)); // west
    assert_eq!(get_col(&result, 2), Cartesian4::new(0.0, 0.0, -1.0, 0.0)); // up
}

#[test]
fn north_west_up_works_at_origin() {
    let origin = Cartesian3::ZERO;
    let result = transforms::north_west_up_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    assert_eq!(get_col(&result, 0), Cartesian4::new(-1.0, 0.0, 0.0, 0.0)); // north
    assert_eq!(get_col(&result, 1), Cartesian4::new(0.0, -1.0, 0.0, 0.0)); // west
    assert_eq!(get_col(&result, 2), Cartesian4::UNIT_Z); // up
}

// ===== headingPitchRollToFixedFrame =====

#[test]
fn heading_pitch_roll_to_fixed_frame_works() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let heading = CesiumMath::to_radians(20.0);
    let pitch = CesiumMath::to_radians(30.0);
    let roll = CesiumMath::to_radians(40.0);
    let hpr = HeadingPitchRoll::new(heading, pitch, roll);

    // Compute expected rotation from quaternion
    let hpr_quat = Quaternion::from_heading_pitch_roll_new(&hpr);
    let expected_rotation = Matrix3::from_quaternion_new(&hpr_quat);
    let mut expected_x = Cartesian3::default();
    let mut expected_y = Cartesian3::default();
    let mut expected_z = Cartesian3::default();
    Matrix3::get_column(&expected_rotation, 0, &mut expected_x);
    Matrix3::get_column(&expected_rotation, 1, &mut expected_y);
    Matrix3::get_column(&expected_rotation, 2, &mut expected_z);

    // Apply the same permutation as the JS test: fromElements(z, x, y)
    let expected_x = Cartesian3::new(expected_x.z, expected_x.x, expected_x.y);
    let expected_y = Cartesian3::new(expected_y.z, expected_y.x, expected_y.y);
    let expected_z = Cartesian3::new(expected_z.z, expected_z.x, expected_z.y);

    let result = transforms::heading_pitch_roll_to_fixed_frame_new(
        &origin,
        &hpr,
        Some(&Ellipsoid::UNIT_SPHERE),
    );

    let col0 = get_col(&result, 0);
    let col1 = get_col(&result, 1);
    let col2 = get_col(&result, 2);
    let col3 = get_col(&result, 3);

    let actual_x = Cartesian3::new(col0.x, col0.y, col0.z);
    let actual_y = Cartesian3::new(col1.x, col1.y, col1.z);
    let actual_z = Cartesian3::new(col2.x, col2.y, col2.z);

    assert!(Cartesian3::equals_epsilon(Some(&actual_x), Some(&expected_x), Some(CesiumMath::EPSILON14), None));
    assert!(Cartesian3::equals_epsilon(Some(&actual_y), Some(&expected_y), Some(CesiumMath::EPSILON14), None));
    assert!(Cartesian3::equals_epsilon(Some(&actual_z), Some(&expected_z), Some(CesiumMath::EPSILON14), None));
    assert!(Cartesian3::equals_epsilon(
        Some(&Cartesian3::new(col3.x, col3.y, col3.z)),
        Some(&origin),
        Some(CesiumMath::EPSILON14),
        None,
    ));
}

// ===== headingPitchRollQuaternion =====

#[test]
fn heading_pitch_roll_quaternion_works() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let heading = CesiumMath::to_radians(20.0);
    let pitch = CesiumMath::to_radians(30.0);
    let roll = CesiumMath::to_radians(40.0);
    let hpr = HeadingPitchRoll::new(heading, pitch, roll);

    let transform = transforms::heading_pitch_roll_to_fixed_frame_new(
        &origin,
        &hpr,
        Some(&Ellipsoid::UNIT_SPHERE),
    );
    let expected = Matrix4::get_matrix3_new(&transform);

    let quat = transforms::heading_pitch_roll_quaternion_new(
        &origin,
        &hpr,
        Some(&Ellipsoid::UNIT_SPHERE),
    );
    let actual = Matrix3::from_quaternion_new(&quat);

    assert!(Matrix3::equals_epsilon(&actual, &expected, CesiumMath::EPSILON11));
}

// ===== fixedFrameToHeadingPitchRoll =====

#[test]
fn fixed_frame_to_heading_pitch_roll_roundtrip() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let heading = CesiumMath::to_radians(20.0);
    let pitch = CesiumMath::to_radians(30.0);
    let roll = CesiumMath::to_radians(40.0);
    let hpr = HeadingPitchRoll::new(heading, pitch, roll);

    let transform = transforms::heading_pitch_roll_to_fixed_frame_new(
        &origin,
        &hpr,
        Some(&Ellipsoid::UNIT_SPHERE),
    );

    let recovered = transforms::fixed_frame_to_heading_pitch_roll_new(
        &transform,
        Some(&Ellipsoid::UNIT_SPHERE),
    );

    assert!((recovered.heading - heading).abs() < CesiumMath::EPSILON10);
    assert!((recovered.pitch - pitch).abs() < CesiumMath::EPSILON10);
    assert!((recovered.roll - roll).abs() < CesiumMath::EPSILON10);
}

#[test]
fn fixed_frame_to_heading_pitch_roll_at_zero_origin() {
    // When center is ZERO, should return zero HPR
    let mut transform = Matrix4::default();
    transforms::east_north_up_to_fixed_frame(&Cartesian3::ZERO, Some(&Ellipsoid::WGS84), &mut transform);
    // Override translation to zero
    let mut transform2 = Matrix4::default();
    Matrix4::set_translation(&transform, &Cartesian3::ZERO, &mut transform2);

    let hpr = transforms::fixed_frame_to_heading_pitch_roll_new(&transform2, Some(&Ellipsoid::WGS84));
    assert_eq!(hpr.heading, 0.0);
    assert_eq!(hpr.pitch, 0.0);
    assert_eq!(hpr.roll, 0.0);
}

// ===== localFrameToFixedFrame consistency =====

#[test]
fn local_frame_consistency_all_axes() {
    // Verify that all generated local frames are consistent with ENU
    let positions = [
        Cartesian3::new(0.0, 0.0, 1.0),
        Cartesian3::new(0.0, 0.0, -1.0),
        Cartesian3::new(10.0, 20.0, 30.0),
        Cartesian3::new(-10.0, -20.0, -30.0),
    ];

    for pos in &positions {
        let enu = transforms::east_north_up_to_fixed_frame_new(pos, Some(&Ellipsoid::UNIT_SPHERE));
        let enu_col3 = get_col(&enu, 3);

        // Test a few frame combinations
        let ned = transforms::north_east_down_to_fixed_frame_new(pos, Some(&Ellipsoid::UNIT_SPHERE));
        // NED: col0=north=ENU col1, col1=east=ENU col0, col2=down=-ENU col2
        assert!(Cartesian4::equals_epsilon(Some(&get_col(&ned, 0)), Some(&get_col(&enu, 1)), Some(CesiumMath::EPSILON14), None));
        assert!(Cartesian4::equals_epsilon(Some(&get_col(&ned, 1)), Some(&get_col(&enu, 0)), Some(CesiumMath::EPSILON14), None));
        // Translation should always match
        assert!(Cartesian4::equals_epsilon(Some(&get_col(&ned, 3)), Some(&enu_col3), Some(CesiumMath::EPSILON14), None));
    }
}

// ===== invalid axis combinations =====

#[test]
fn invalid_axis_combination_returns_false() {
    let origin = Cartesian3::new(1.0, 0.0, 0.0);
    let mut result = Matrix4::default();
    // Same axis twice should fail
    let ok = transforms::local_frame_to_fixed_frame(
        &origin,
        Some(&Ellipsoid::UNIT_SPHERE),
        transforms::AxisDirection::North,
        transforms::AxisDirection::North,
        &mut result,
    );
    assert!(!ok);
}

// ===== general position test =====

#[test]
fn east_north_up_general_position() {
    let origin = Cartesian3::new(1.0, 2.0, 3.0);
    let result = transforms::east_north_up_to_fixed_frame_new(&origin, Some(&Ellipsoid::UNIT_SPHERE));

    // Translation should be origin
    let col3 = get_col(&result, 3);
    assert_eq!(col3, Cartesian4::new(1.0, 2.0, 3.0, 1.0));

    // East should be perpendicular to up (surface normal)
    let east = Cartesian3::new(get_col(&result, 0).x, get_col(&result, 0).y, get_col(&result, 0).z);
    let up = Cartesian3::new(get_col(&result, 2).x, get_col(&result, 2).y, get_col(&result, 2).z);
    let dot = Cartesian3::dot(&east, &up);
    assert!(dot.abs() < CesiumMath::EPSILON14);

    // All columns should be unit vectors
    assert!((Cartesian3::magnitude(&east) - 1.0).abs() < CesiumMath::EPSILON14);
    let north = Cartesian3::new(get_col(&result, 1).x, get_col(&result, 1).y, get_col(&result, 1).z);
    assert!((Cartesian3::magnitude(&north) - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((Cartesian3::magnitude(&up) - 1.0).abs() < CesiumMath::EPSILON14);
}

// ===== computeIcrfToMoonFixedMatrix =====
//
// DEVIATION: the JS `throws if the date parameter is not specified` cases are
// not mirrored because the Rust type system makes the `date` parameter
// required.

#[test]
fn icrf_to_moon_fixed_matrix_works() {
    // 2011-07-03 00:00:00 UTC
    let mut time = JulianDate::new(2455745.0, 43200.0, TimeStandard::UTC);

    let mut result_t = Matrix3::default();
    let t_ptr = {
        let t = transforms::compute_icrf_to_moon_fixed_matrix(&time, &mut result_t);
        t as *const Matrix3
    };
    assert_eq!(t_ptr, &result_t as *const Matrix3);
    let t = &result_t;

    // rotation matrix determinants are 1.0
    let det = Matrix3::determinant(t);
    assert!((det - 1.0).abs() < CesiumMath::EPSILON14);

    // rotation matrix inverses are equal to its transpose
    let t4 = Matrix4::from_rotation_translation_new(t, &Cartesian3::ZERO);
    let inverse = Matrix4::inverse_new(&t4).unwrap();
    let mut inverse_transformation = Matrix4::default();
    Matrix4::inverse_transformation(&t4, &mut inverse_transformation);
    assert!(Matrix4::equals_epsilon(
        &inverse,
        &inverse_transformation,
        CesiumMath::EPSILON14
    ));

    // add one sidereal month
    time = JulianDate::add_hours(&time, 27.321661 * 24.0);
    let mut result_u = Matrix3::default();
    let u_ptr = {
        let u = transforms::compute_icrf_to_moon_fixed_matrix(&time, &mut result_u);
        u as *const Matrix3
    };
    assert_eq!(u_ptr, &result_u as *const Matrix3);
    let u = &result_u;
    let t_angle = Quaternion::compute_angle(&Quaternion::from_rotation_matrix_new(t));
    let u_angle = Quaternion::compute_angle(&Quaternion::from_rotation_matrix_new(u));
    assert!((t_angle - u_angle).abs() < CesiumMath::EPSILON3);

    // The JS Matrix3 constructor stores its arguments in column-major order.
    let expected_mtx = Matrix3::new(
        -0.44796811269393627,
        0.8934634849604557,
        0.03236620230657612,
        0.8184479558129512,
        0.3952490953922868,
        0.4170384828971786,
        0.3598159441089767,
        0.2133099942194372,
        -0.9083123541662688,
    );

    let transposed = Matrix3::transpose_new(t);
    let test_inverse = Matrix3::multiply_new(&transposed, &expected_mtx);
    assert!(Matrix3::equals_epsilon(
        &test_inverse,
        &Matrix3::IDENTITY,
        CesiumMath::EPSILON14
    ));
    let mut test_diff = Matrix3::default();
    for i in 0..9 {
        test_diff.elements[i] = t.elements[i] - expected_mtx.elements[i];
    }
    assert!(Matrix3::equals_epsilon(
        &test_diff,
        &Matrix3::default(),
        CesiumMath::EPSILON14
    ));
}

// ===== computeIcrfToFixedMatrix =====
//
// DEVIATION: the JS `works with data from STK Components`, `works with
// hard-coded data`, `works over day boundary` and `works over day boundary
// backwards` cases download EOP/XYS data over the network
// (`EarthOrientationParameters.fromUrl` / `Iau2006XysData` chunk downloads)
// and are not mirrored; the Rust port performs no network I/O.

#[test]
fn icrf_to_fixed_returns_undefined_before_xys_data_is_loaded() {
    // Mirrors "returns undefined before XYS data is loaded": with a fresh
    // (never-loaded) Iau2006XysData the computation must yield undefined.
    // The Rust module-level XYS data is likewise never loaded.
    let time = JulianDate::new(2455745.0, 43200.0, TimeStandard::UTC);
    let mut result = Matrix3::default();
    assert!(transforms::compute_icrf_to_fixed_matrix(&time, &mut result).is_none());
    assert!(transforms::compute_fixed_to_icrf_matrix(&time, &mut result).is_none());
}

// ===== pointToGLWindowCoordinates / pointToWindowCoordinates =====
//
// DEVIATION: the JS `throws without ...` cases are not mirrored because the
// Rust type system makes all parameters required.

const WINDOW_TEST_WIDTH: f64 = 1024.0;
const WINDOW_TEST_HEIGHT: f64 = 768.0;

fn window_test_fixtures() -> (Matrix4, Matrix4) {
    let perspective = Matrix4::compute_perspective_field_of_view_new(
        CesiumMath::to_radians(60.0),
        WINDOW_TEST_WIDTH / WINDOW_TEST_HEIGHT,
        1.0,
        10.0,
    );
    let viewport = Viewport {
        x: 0.0,
        y: 0.0,
        width: WINDOW_TEST_WIDTH,
        height: WINDOW_TEST_HEIGHT,
    };
    let vp_transform =
        Matrix4::compute_viewport_transformation_new(Some(&viewport), Some(0.0), Some(1.0));
    (perspective, vp_transform)
}

fn window_test_view() -> Matrix4 {
    Matrix4::from_camera_new(&CameraView {
        position: Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, 2.0),
        direction: Cartesian3::negate_new(&Cartesian3::UNIT_X),
        up: Cartesian3::UNIT_Z,
    })
}

#[test]
fn point_to_gl_window_coordinates_works_at_the_center() {
    let (perspective, vp_transform) = window_test_fixtures();
    let view = window_test_view();
    let mvp_matrix = Matrix4::multiply_new(&perspective, &view);

    let expected = Cartesian2::new(WINDOW_TEST_WIDTH * 0.5, WINDOW_TEST_HEIGHT * 0.5);
    let returned_result = transforms::point_to_gl_window_coordinates_new(
        &mvp_matrix,
        &vp_transform,
        &Cartesian3::ZERO,
    );
    assert_eq!(returned_result, expected);
}

#[test]
fn point_to_gl_window_coordinates_works_with_a_result_parameter() {
    let (perspective, vp_transform) = window_test_fixtures();
    let view = window_test_view();
    let mvp_matrix = Matrix4::multiply_new(&perspective, &view);

    let expected = Cartesian2::new(WINDOW_TEST_WIDTH * 0.5, WINDOW_TEST_HEIGHT * 0.5);
    let mut result = Cartesian2::default();
    let returned_ptr = {
        let returned_result = transforms::point_to_gl_window_coordinates(
            &mvp_matrix,
            &vp_transform,
            &Cartesian3::ZERO,
            &mut result,
        );
        returned_result as *const Cartesian2
    };
    assert_eq!(returned_ptr, &result as *const Cartesian2);
    assert_eq!(result, expected);
}

#[test]
fn point_to_gl_window_coordinates_works_at_the_lower_left() {
    let (perspective, vp_transform) = window_test_fixtures();
    // COLUMN3ROW2 = elements[14], COLUMN2ROW2 = elements[10],
    // COLUMN0ROW0 = elements[0], COLUMN1ROW1 = elements[5].
    let z = -perspective.elements[14] / perspective.elements[10];
    let x = z / perspective.elements[0];
    let y = z / perspective.elements[5];
    let point = Cartesian3::new(x, y, z);

    let expected = Cartesian2::new(0.0, 0.0);
    let returned_result =
        transforms::point_to_gl_window_coordinates_new(&perspective, &vp_transform, &point);
    assert!(Cartesian2::equals_epsilon(
        Some(&returned_result),
        Some(&expected),
        None,
        Some(CesiumMath::EPSILON12),
    ));
}

#[test]
fn point_to_gl_window_coordinates_works_at_the_upper_right() {
    let (perspective, vp_transform) = window_test_fixtures();
    let z = -perspective.elements[14] / perspective.elements[10];
    let x = -z / perspective.elements[0];
    let y = -z / perspective.elements[5];
    let point = Cartesian3::new(x, y, z);
    let expected = Cartesian2::new(WINDOW_TEST_WIDTH, WINDOW_TEST_HEIGHT);

    let returned_result =
        transforms::point_to_gl_window_coordinates_new(&perspective, &vp_transform, &point);
    assert!(Cartesian2::equals_epsilon(
        Some(&returned_result),
        Some(&expected),
        None,
        Some(CesiumMath::EPSILON12),
    ));
}

#[test]
fn point_to_window_coordinates_works_at_the_center() {
    let (perspective, vp_transform) = window_test_fixtures();
    let view = window_test_view();
    let mvp_matrix = Matrix4::multiply_new(&perspective, &view);

    let expected = Cartesian2::new(WINDOW_TEST_WIDTH * 0.5, WINDOW_TEST_HEIGHT * 0.5);
    let returned_result =
        transforms::point_to_window_coordinates_new(&mvp_matrix, &vp_transform, &Cartesian3::ZERO);
    assert_eq!(returned_result, expected);
}

#[test]
fn point_to_window_coordinates_works_with_a_result_parameter() {
    let (perspective, vp_transform) = window_test_fixtures();
    let view = window_test_view();
    let mvp_matrix = Matrix4::multiply_new(&perspective, &view);

    let expected = Cartesian2::new(WINDOW_TEST_WIDTH * 0.5, WINDOW_TEST_HEIGHT * 0.5);
    let mut result = Cartesian2::default();
    let returned_ptr = {
        let returned_result = transforms::point_to_window_coordinates(
            &mvp_matrix,
            &vp_transform,
            &Cartesian3::ZERO,
            &mut result,
        );
        returned_result as *const Cartesian2
    };
    assert_eq!(returned_ptr, &result as *const Cartesian2);
    assert_eq!(result, expected);
}

#[test]
fn point_to_window_coordinates_works_at_the_lower_left() {
    let (perspective, vp_transform) = window_test_fixtures();
    let z = -perspective.elements[14] / perspective.elements[10];
    let x = z / perspective.elements[0];
    let y = z / perspective.elements[5];
    let point = Cartesian3::new(x, y, z);

    let expected = Cartesian2::new(0.0, WINDOW_TEST_HEIGHT);
    let returned_result =
        transforms::point_to_window_coordinates_new(&perspective, &vp_transform, &point);
    assert!(Cartesian2::equals_epsilon(
        Some(&returned_result),
        Some(&expected),
        None,
        Some(CesiumMath::EPSILON12),
    ));
}

#[test]
fn point_to_window_coordinates_works_at_the_upper_right() {
    let (perspective, vp_transform) = window_test_fixtures();
    let z = -perspective.elements[14] / perspective.elements[10];
    let x = -z / perspective.elements[0];
    let y = -z / perspective.elements[5];
    let point = Cartesian3::new(x, y, z);
    let expected = Cartesian2::new(WINDOW_TEST_WIDTH, 0.0);

    let returned_result =
        transforms::point_to_window_coordinates_new(&perspective, &vp_transform, &point);
    assert!(Cartesian2::equals_epsilon(
        Some(&returned_result),
        Some(&expected),
        None,
        Some(CesiumMath::EPSILON12),
    ));
}

// ===== basisTo2D =====
//
// DEVIATION: the JS `throws without projection/matrix/result` cases are not
// mirrored because the Rust type system makes all parameters required.

fn basis_to_2d_fixture() -> (Ellipsoid, GeographicProjection, Cartesian3, Matrix4) {
    let ellipsoid = Ellipsoid::WGS84;
    let projection = GeographicProjection::new(Some(ellipsoid));
    let origin = Cartesian3::from_degrees_new(-72.0, 40.0, Some(100.0), None);
    let heading = CesiumMath::to_radians(90.0);
    let pitch = CesiumMath::to_radians(45.0);
    let roll = 0.0;
    let hpr = HeadingPitchRoll::new(heading, pitch, roll);

    let mut model_matrix = Matrix4::default();
    transforms::heading_pitch_roll_to_fixed_frame(
        &origin,
        &hpr,
        Some(&ellipsoid),
        &mut model_matrix,
    );
    (ellipsoid, projection, origin, model_matrix)
}

#[test]
fn basis_to_2d_projects_translation() {
    let (ellipsoid, projection, origin, model_matrix) = basis_to_2d_fixture();

    let mut model_matrix_2d = Matrix4::default();
    transforms::basis_to_2d(&projection, &model_matrix, &mut model_matrix_2d);

    let mut translation_2d = Cartesian3::default();
    let mut column3 = Cartesian4::default();
    Matrix4::get_column(&model_matrix_2d, 3, &mut column3);
    Cartesian3::from_cartesian4(&column3, &mut translation_2d);

    let mut carto = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(&origin, &mut carto);
    let mut expected = projection.project(&carto);
    let expected_swapped = Cartesian3::new(expected.z, expected.x, expected.y);
    expected = expected_swapped;

    assert!(Cartesian3::equals(Some(&translation_2d), Some(&expected)));
}

#[test]
fn basis_to_2d_transforms_rotation() {
    let (ellipsoid, projection, origin, model_matrix) = basis_to_2d_fixture();

    let mut model_matrix_2d = Matrix4::default();
    transforms::basis_to_2d(&projection, &model_matrix, &mut model_matrix_2d);

    let rotation_2d = Matrix4::get_matrix3_new(&model_matrix_2d);

    let mut enu = Matrix4::default();
    transforms::east_north_up_to_fixed_frame(&origin, Some(&ellipsoid), &mut enu);
    let mut enu_inverse = Matrix4::default();
    Matrix4::inverse_transformation(&enu, &mut enu_inverse);

    let hpr_plus_translate = Matrix4::multiply_new(&enu_inverse, &model_matrix);
    let hpr2 = Matrix4::get_matrix3_new(&hpr_plus_translate);

    // Mirror the JS row permutation: row0 <- row2, row1 <- row0, row2 <- row1.
    // Column-major elements: row r of column c lives at elements[c * 3 + r].
    let mut expected = Matrix3::default();
    for c in 0..3 {
        expected.elements[c * 3 + 0] = hpr2.elements[c * 3 + 2];
        expected.elements[c * 3 + 1] = hpr2.elements[c * 3 + 0];
        expected.elements[c * 3 + 2] = hpr2.elements[c * 3 + 1];
    }

    assert!(Matrix3::equals_epsilon(
        &rotation_2d,
        &expected,
        CesiumMath::EPSILON3
    ));
}

// ===== ellipsoidTo2DModelMatrix =====
//
// DEVIATION: the JS `throws without projection/center/result` cases are not
// mirrored because the Rust type system makes all parameters required.

#[test]
fn ellipsoid_to_2d_model_matrix_creates_model_matrix_to_transform_vertices_centered_origin_to_2d() {
    let ellipsoid = Ellipsoid::WGS84;
    let projection = GeographicProjection::new(Some(ellipsoid));
    let origin = Cartesian3::from_degrees_new(-72.0, 40.0, Some(100.0), None);

    let mut actual = Matrix4::default();
    transforms::ellipsoid_to_2d_model_matrix(&projection, &origin, &mut actual);

    let mut expected = Matrix4::default();
    Matrix4::from_translation(&origin, &mut expected);
    let expected_snapshot = expected;
    transforms::basis_to_2d(&projection, &expected_snapshot, &mut expected);

    let actual_rotation = Matrix4::get_matrix3_new(&actual);
    let expected_rotation = Matrix4::get_matrix3_new(&expected);
    assert!(Matrix3::equals_epsilon(
        &actual_rotation,
        &expected_rotation,
        CesiumMath::EPSILON14
    ));

    let mut from_enu = Matrix4::default();
    transforms::east_north_up_to_fixed_frame(&origin, Some(&ellipsoid), &mut from_enu);
    let mut to_enu = Matrix4::default();
    Matrix4::inverse_transformation(&from_enu, &mut to_enu);

    // JS uses Matrix4.getTranslation (a Cartesian4); the translation is the
    // fourth column with w == 1.
    let mut to_enu_translation = Cartesian4::default();
    Matrix4::get_column(&to_enu, 3, &mut to_enu_translation);
    let mut projected_translation = Cartesian4::default();
    Matrix4::get_column(&expected, 3, &mut projected_translation);

    let expected_translation = Cartesian4::new(
        projected_translation.x + to_enu_translation.z,
        projected_translation.y + to_enu_translation.x,
        projected_translation.z + to_enu_translation.y,
        projected_translation.w,
    );

    let mut actual_translation = Cartesian4::default();
    Matrix4::get_column(&actual, 3, &mut actual_translation);

    assert!(Cartesian4::equals_epsilon(
        Some(&actual_translation),
        Some(&expected_translation),
        None,
        Some(CesiumMath::EPSILON14),
    ));
}
