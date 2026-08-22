//! Mirrors packages/engine/Specs/Core/TransformsSpec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::heading_pitch_roll::HeadingPitchRoll;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::quaternion::Quaternion;
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
